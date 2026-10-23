> **13 October member-client milestone completed:** independent byte-derived review, signed approvals/rejections, real creation/payment receipts and actual local sequencer inclusion. Read [client/CLIENT_RESULTS_2026-10-13.md](client/CLIENT_RESULTS_2026-10-13.md) and [client/README.md](client/README.md). Public devnet and full product delivery remain pending.

> **Latest checkpoint — 12 October 2026:** Real creation and 5-of-7 spend proofs now verify, and an unchanged pinned standalone sequencer included private funding, vault creation and the spend. Read [REAL_RESULTS.md](REAL_RESULTS.md) and [REAL_WORKFLOW.md](REAL_WORKFLOW.md). The older execution-only statements below describe the historical 7 October checkpoint.

# Vault creation and member enrollment

This workspace adds initialization to the earlier private LEZ vault prototype. All initial members sign one canonical vault descriptor, the private PDA address commits to that descriptor, and the program rejects an existing configuration. Creation can include an atomic deposit from a separately authorized funding account. Subsequent transfers reuse the archived M-of-N verifier and committed state rules.

The executed fixture creates a 5-of-7 vault with seven enrollment signatures, deposits 500 test units, then sends 37 to a fresh private recipient using five transfer approvals. Both transactions execute through the pinned LEZ privacy circuit; all four encrypted output notes match the expected accounts. Forty-five invalid apply/plan cases are rejected by both native Rust and the actual guest binary, and two invalid funding flows abort without a PPE output. Six read-only RPC preflight tests pass.

The October 12 baseline and October 13 member-client workflow both have real proof and actual local sequencer evidence. The original execution-only reports remain as historical checkpoints. The latest member workflow uses seven fresh local test signing keys; LEZ private-account material remains public test fixtures. Read the dated client results for exact provenance and remaining limits.

Read `client/CLIENT_RESULTS_2026-10-13.md` for the current evidence, `client/README.md` for the member journey and reproduction, `DESIGN.md` for the enrollment contract, and `DEVNET.md` for public-network work. `reports/RESULTS.md` is the historical execution-only checkpoint.

## Reproduce

Use the Rust/RISC Zero versions in the parent README. Keep `lez-upstream` beside `lez-approval-bench`, at commit `411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4`. The host and guest lockfiles preserve the integration workspace's external dependency versions.

```sh
cd lez-approval-bench/lifecycle
CARGO_TARGET_DIR=../integration/target CARGO_BUILD_JOBS=2 RISC0_BUILD_LOCKED=1 \
  cargo build --release --locked -p lifecycle-host
python3 scripts/run.py validate --report-dir reports/reproduction-validation \
  --r0vm /absolute/path/to/r0vm
python3 scripts/run.py execute --report-dir reports/reproduction-execution \
  --r0vm /absolute/path/to/r0vm
python3 -m unittest discover -s scripts -p 'test_*.py' -v
python3 scripts/devnet_preflight.py --output reports/reproduction-preflight.json
```

The preflight deliberately exits with status 2 while readiness blockers remain. It writes the complete report first. It makes no network call unless an explicit endpoint and expected channel are supplied together. See `DEVNET.md`.

The runner sets `RISC0_DEV_MODE=0`, executes with the installed `r0vm`, records hardware/resources, and refuses to overwrite trial logs. The host refuses to overwrite completed result JSON. Choose fresh report directories when reproducing. The shared Cargo target directory avoids rebuilding existing dependencies; original integration source, binaries and saved evidence are unchanged.

## Files and scope

| Path | Purpose |
| --- | --- |
| `core/src/lib.rs` | Descriptor, address derivation, unanimous enrollment and initialization dispatcher |
| `methods/guest` | Actual RISC Zero guest for creation and transfer |
| `host/src/fixture.rs` | Public deterministic private-account fixtures and exact note checks |
| `host/src/validation.rs` | Positive comparisons and 47 rejection cases |
| `host/src/main.rs` | Execution, real proving and independent saved-file verification modes |
| `client/` | Canonical-byte review, local signing, signed response collection and exact instruction exports |
| `scripts/devnet_preflight.py` | Pinned-source checks and optional read-only RPC queries |
| `reports/lifecycle/` | Checked creation/spend outputs, canonical fixture descriptor/instruction, guest and measurements |
| `reports/validation/` | Validation results, exact guest and retained trial logs |

The transfer core and instrumented LEE execution adapter are imported from the unchanged `../integration/` source files. They are not duplicated or edited. Original project code is MIT OR Apache-2.0 under the parent licenses. External package versions and sources are identical to the integration lockfiles; its `THIRD-PARTY-NOTICES.md` and `third-party-licenses/` inventory apply here too.

Enrollment currently proves possession of the listed Schnorr signing keys. It does not implement a private LEZ-account ownership handshake, secure sharing of viewing/nullifier secrets, an encrypted invitation service, key rotation, chat, QML, or a production signing UI. The default fixture still signs with public test keys. The member client signs with separately generated local development keys; its saved reports contain only public responses. None of the delivered LEZ fixture material may secure real funds.
