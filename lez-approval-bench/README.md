> **13 October member-client milestone completed:** seven fresh local test signers, signed approvals/rejections, unchanged-guest real proofs and actual standalone-sequencer inclusion. Start with [lifecycle/client/CLIENT_RESULTS_2026-10-13.md](lifecycle/client/CLIENT_RESULTS_2026-10-13.md). Public devnet and full product delivery remain pending.

> **12 October update:** The lifecycle guest now has real proofs and actual standalone-sequencer inclusion for the controlled create/fund/spend journey. Start with [lifecycle/REAL_RESULTS.md](lifecycle/REAL_RESULTS.md). Public devnet and full product delivery remain pending. Earlier checkpoint descriptions below are historical.

# LEZ multisig feasibility prototypes

Prepared for Peter Nguyen's RFP-005 application, 7 October 2026.

The private-account integration is in `integration/`. Its real 5-of-7 private PDA transfer proof completed in 46.99 minutes, producing a 232,072-byte succinct transaction envelope. A fresh process verified the saved transaction through pinned LEZ state transition checks. Read `integration/reports/RESULTS.md` for that proof and its local-only scope. The earlier standalone result below is preserved.

The new `lifecycle/` workspace adds unanimous member enrollment, descriptor-bound creation and optional atomic funding. Creation followed by a 5-of-7 transfer passes actual guest/PPE execution and 47 rejection cases. Real proofs and local node inclusion were recorded on October 12 and repeated through the member client on October 13. Start with `lifecycle/client/README.md` and its dated results; `lifecycle/reports/RESULTS.md` is the original execution-only checkpoint.


This standalone Rust/RISC Zero project implements an M-of-N approval check and a harness for real zkVM proving. A real 5-of-7 composite proof completed in 642.3 seconds and its saved receipt was verified in a separate process. The guest uses pinned official RISC Zero acceleration patches; the earlier interrupted baseline trial remains in the evidence. It is an initial engineering artifact, not a deployed multisig or completion of the RFP's performance requirements. Read `reports/RESULTS.md` for the recorded outcome and limitations.

## What is implemented

- BIP-340 Schnorr verification using `k256 0.13.4` and its prehashed verification API, with official acceleration patches applied only to the guest.
- A configuration with a threshold, an epoch and distinct member public keys.
- Distinct-member approval counting with validation of keys, indices and signature sizes.
- Domain-separated SHA-256 commitments binding the configuration, network, instance, program, proposal, destination, expiry value and instruction bytes.
- A RISC Zero 3.0.5 guest that verifies the approvals and commits a result to its journal.
- A local host that refuses development-mode receipts, verifies the receipt against the compiled guest image ID, and checks the journal against the fixture.

`core/` contains the verifier and native tests; `methods/guest/` is the guest entrypoint; `host/` proves or verifies receipts; `scripts/measure.py` captures one proof run and resource observations. Both Cargo lockfiles are included.

## Reproduce

Use an x86-64 Linux machine with enough available memory. The recorded machine has an 8 GiB memory limit. No GPU, account, RPC endpoint, wallet, or paid proving service is used.

Install Rust and the official [RISC Zero tools](https://dev.risczero.com/api/zkvm/install). Pin these versions rather than choosing `latest`:

```sh
rustup toolchain install 1.98.1 --profile minimal
rzup install rust 1.94.1
rzup install r0vm 3.0.5
rzup default rust 1.94.1
rzup default r0vm 3.0.5
```

From this project's root, build and verify the included receipt:

```sh
cargo test --release --locked -p approval-core
CARGO_BUILD_JOBS=2 RISC0_BUILD_LOCKED=1 cargo build --release --locked -p approval-host
target/release/approval-host --m 5 --n 7 \
  --verify-receipt reports/accelerated/m5-n7.receipt \
  --guest-program reports/accelerated/approval-guest.bin
python3 scripts/check_saved_receipt.py --report-dir reports/accelerated
```

The final command checks the original receipt, the wrong expected threshold, the wrong guest image and an altered journal. It leaves the original receipt file unchanged. The host is compiled with `disable-dev-mode`, refuses fake receipts, verifies the selected program image and compares the journal with the independent native fixture.

To repeat validation and generate a new proof, choose a fresh report directory:

```sh
mkdir -p reports/reproduction
cp target/riscv-guest/approval-methods/approval-guest/riscv32im-risc0-zkvm-elf/release/approval-guest.bin \
  reports/reproduction/approval-guest.bin
RISC0_DEV_MODE=0 target/release/approval-host --mode validate --r0vm r0vm \
  --out reports/reproduction/backend.json
RISC0_DEV_MODE=0 target/release/approval-host --mode execute --m 5 --n 7 --r0vm r0vm \
  --out reports/reproduction/m5-n7.json
python3 scripts/measure.py --m 5 --n 7 --threads 8 --r0vm r0vm \
  --report-dir reports/reproduction --timeout-seconds 1800
python3 scripts/check_saved_receipt.py --report-dir reports/reproduction
```

The measurement wrapper refuses to overwrite a previous trial, uses real local proving, and records timeouts and interruptions. Its timeout stops the local host/prover process group. The successful recorded run used eight CPU workers and produced a 2,268,522-byte composite receipt. The host also accepts `--receipt succinct`; compression has not been benchmarked here.

The `validate` mode executes a separate validation guest. It checks 21 approval cases against the host's unpatched implementation and 15 published BIP-340 vectors against the accelerated backend. This is explicitly execution-only validation; its program is distinct from the guest used for the real approval proof.

`reports/accelerated/approval-guest.bin` is the exact proved guest program, while `reports/approval-guest.bin` is the original baseline. `--guest-program` computes the supplied binary's image ID, so the included receipt can be checked even if a rebuild produces a different ID. Different compiler, dependency or build-path inputs can change an image ID. Both lockfiles and all acceleration commits are pinned; `RISC0_BUILD_LOCKED=1` pins guest resolution as well as the host's `--locked` build.

The installed runtime was restored from a checksum-verified official release archive after an earlier truncated download. Those startup failures and the interrupted unaccelerated proof are retained separately. Exact runtime/compiler download URLs, checksums, patch commits and guest/receipt hashes are in `sources.json`.

## What this does not establish

The guest receives its configuration from the caller. A consuming LEZ program must bind the returned configuration commitment to authoritative current account state, and the action commitment to the transaction actually executed. This prototype has no such integration. The journal exposes the threshold, member count and signer indices; it is not a finished private disclosure design.

An expiry value and proposal ID are signed, but there is no trusted clock, expiry enforcement or replay database. No asset is held or transferred. Account ownership, private accounts, custody, transaction atomicity, timelocks, spending limits, member removal, chat, QML, CLI product flows and selective disclosure remain unimplemented. There is no independent audit.

The inspected LEZ source uses the same top-level `risc0-zkvm` and `k256` versions and Schnorr prehash API. This project has its own transitive dependency lockfile and application message format; it is not a build of that LEZ revision and does not reproduce its full execution wrapper or optimizations. Do not interpret the serialized proof size as a LEZ transaction size or infer devnet block fit from this benchmark.

Fixtures deliberately use public deterministic signing keys and a synthetic action. They must never hold assets. The bounded verifier is not a hardened service input decoder; deserialization occurs before its semantic size checks.

## Current engineering gate

The separate integration and lifecycle workspaces advance the original private-account gate. Next, prove the new creation flow and its subsequent spend, ingest real network witnesses, and verify sequencer acceptance on a confirmed deployment. Member identity/key distribution, revocation, disclosure, full performance scaling and the remaining product requirements still need their own evidence before later milestone commitments.

## License

Original project code is available under either the MIT license or Apache License 2.0, at your option. See `LICENSE-MIT` and `LICENSE-APACHE`. Third-party dependencies retain their licenses; the published Bitcoin test vectors are attributed in `THIRD-PARTY-NOTICES.md`.
