# Treasury member client

> **17 October update:** checkpoint-backed state import and verified transfer
> review/signing are available. Read [STATE_IMPORT.md](STATE_IMPORT.md) for the
> trust boundary, commands, tests and historical-fixture example. The unverified
> local signing command is now explicitly named `respond-offline`.

This Rust library and CLI move approval collection out of the deterministic proving fixture. Members can review a request, produce a signed approval or rejection on their own machine, and return only the response file. The collector verifies responses and exports the exact Borsh instruction consumed by the existing LEZ guest. It never reads a member's secret key.

This is a local development client, with an offline file transport. It is not yet a Logos headless module, a Basecamp mini-app, or a Logos Chat room. The guest remains the authority on execution. The client implements no network submission and does not authenticate a context file as accepted chain state.

## Build

Keep the unchanged upstream checkout at `411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4` next to `lez-approval-bench`. Use host Rust 1.98.1. From `lifecycle/`:

```sh
LIFECYCLE_PREBUILT_GUEST="$PWD/reports/lifecycle/lifecycle-guest.bin" \
  cargo build --release --locked -p treasury-client -p lifecycle-host
LIFECYCLE_PREBUILT_GUEST="$PWD/reports/lifecycle/lifecycle-guest.bin" \
  cargo test --release --locked -p treasury-client
```

The build pins the exact previously proved guest. No approval logic, custody program, or guest image changes in this milestone. This public source package omits the precompiled Linux host/client binaries; use the release binaries built above. `treasury-client --help` lists the positional CLI arguments. `client/scripts/install_r0vm.py` installs the exact CPU executor after verifying both the official release archive and runtime binary digests.

The included `.github/workflows/treasury-client.yml` runs the client/shared verifier tests, fresh local signer demo, unchanged-guest execution and independent archived-proof checks. It uploads only public test reports, with signing keys outside the checkout. These commands were validated locally; a hosted GitHub Actions run has not been performed. Full fresh proving and standalone settlement remain separate measured runs, rather than being implied by the quick CI job.

## Member journey

1. Each member runs `keygen` locally and shares only the public JSON. Key files contain a local development signing secret in hexadecimal. They are created with mode 0600, never printed, and refused by the signer if they have group/public permissions or are symlinks. This is not an encrypted production keystore or hardware-wallet integration.
2. An organiser imports the public vault descriptor and agreed member public keys with `configure`. This gives a fresh salt and a local expected initial context. The descriptor's supplied viewing/nullifier public material is retained; the client does not distribute its secrets or prove LEZ account ownership.
3. The organiser uses `create` for unanimous enrollment or `propose` for a native-token transfer. Amounts are decimal integer base units; floating-point amounts are rejected.
4. For this explicitly offline fixture workflow, each member obtains the context independently, runs `review`, checks the decoded request and network/vault/program, then supplies its `review_digest` to `respond-offline`. The signer recomputes this digest before signing and refuses changed bytes. For checkpoint-backed state, use `review-verified` and `respond-verified` as described in STATE_IMPORT.md. Human review remains the member's responsibility.
5. The organiser uses `collect` to combine signed response files, possibly in several batches. Supply all retained response files when updating the collection. Duplicate and reordered deliveries are tolerated, and each member counts once. `status` recomputes quorum from signatures, rather than trusting a stored approval count.
6. `export` succeeds only after unanimous enrollment or the configured spending quorum. It also runs the unchanged native guest core's configuration Apply check and writes `instruction.borsh`. Export alone does not produce a privacy proof, execute the native-token call, or submit a transaction.

Example for one member; keep the key path on that member's machine:

```sh
treasury-client keygen /private/member.key member.public.json
treasury-client review context.json spend.request.json
treasury-client respond-offline context.json spend.request.json /private/member.key \
  approve REVIEW_DIGEST_FROM_REVIEW member.response.json
```

The other members receive the same context and request but use their own key files. Never send the secret-key file to the collector. Request, response, and export files are created with mode 0600; export directories use mode 0700. These filesystem protections do not provide E2EE, transport privacy, or metadata resistance.

## Exact signing scope

The review decodes `instruction_hex` with the same Borsh types as the guest. It does not accept a proposer-supplied description or decoded amount. Unsupported instruction variants and unknown JSON fields fail closed.

For a transfer, approvals bind the current configuration, network domain, program, vault, epoch, proposal counter, native-token target, recipient, custody seed, amount, and expiry. Large amounts remain strings in the review JSON to preserve every base unit. The registry currently covers only the two archived instructions: treasury creation and native-token transfer; arbitrary target programs remain unsupported.

For enrollment, every member signs the complete descriptor and expiry, even when the eventual spending threshold is lower. Optional inbound funding is **not** authorized by the enrollment signature. Its source account separately authorizes the transaction, and the review explicitly labels this distinction. Changing funding changes the local review digest, while the guest's enrollment digest remains unchanged by design.

A rejection uses its own signature domain. Relabeling it as an approval fails verification. A rejection is an opinion rather than a veto: five approvals and two rejections can satisfy a five-of-seven treasury. Conflicting signed decisions from one member halt client export and require a new request with a new expiry and new signatures. Recreating identical bytes does not resolve the conflict. The current guest consumes only approvals; rejection handling is an off-chain client policy.

## Context and room trust

A context is a local configuration reference. For creation, participants must independently agree its descriptor, member ordering and network settings. For a live spend, configuration and counter must be synchronized from an independently accepted and decrypted vault note. A JSON context or `predicted-context.json` is not such evidence. The predicted context is produced only to explain the expected counter increment; no local export changes chain state.

The collector is untrusted for signature validity. It can withhold or censor messages and present different partial collections. It cannot turn a rejection into an approval, count a replay twice, change a signed transfer, or supply an approval from a nonmember without failing verification. Availability and reconciliation of different views still require the future coordination-room adapter.

The guest's network field remains an application assertion. A matching domain is not protocol-level protection against identical cloned chains. Wallet network selection, authenticated state synchronization, LEZ-account-to-signing-key ownership, LEZ-to-chat identity binding, secure viewing/nullifier key distribution, and Logos Chat room provisioning are pending.

## Replay the local demo

From `lifecycle/`, choose two new directories. The key directory must be outside the entire deliverable project. The demo generates seven random local test signing keys and deliberately retains public test-only LEZ account material.

```sh
python3 client/scripts/demo.py \
  --client "$PWD/target/release/treasury-client" \
  --key-dir /absolute/private-scratch/new-demo-keys \
  --report-dir "$PWD/client/reports/new-member-demo"
```

The demo gathers seven enrollments and five nonconsecutive spending approvals, records two signed rejections, delivers approval messages repeatedly and out of order, and runs six negative CLI controls. Secret-key files are excluded from the project and its saved reports. Fresh runs have different keys, salts and proof inputs.

## Send the exports into the proof engine

The local bridge retains the historical test funder, recipient, 500-unit deposit, 37-unit spend, initial counter, private-account material, and test expiry. The member set and spending threshold come from the imported enrollment. It refuses incompatible private-account material or other transfer parameters before proving. This is a bounded integration path, not a general wallet adapter.

```sh
python3 scripts/run.py execute --state-profile standalone \
  --host "$PWD/target/release/lifecycle-host" --r0vm /absolute/path/to/r0vm \
  --create-instruction "$PWD/client/reports/new-member-demo/create-export/instruction.borsh" \
  --transfer-instruction "$PWD/client/reports/new-member-demo/spend-export/instruction.borsh" \
  --report-dir reports/new-client-execution
```

For actual proofs, replace `execute` with `succinct` and choose a fresh proof directory. Verify in a separate process with `check-succinct`, the same state profile, and the same two instruction arguments. The historical `validate` suite intentionally retains its original fixtures and refuses imports. See `../REAL_WORKFLOW.md` for the proving toolchain and standalone genesis snapshot.

After real proofs and their independent check finish, `scripts/settle_local.py` accepts the same two instruction arguments. It verifies their hashes against the proved report and passes them to the node-witness checker. No remote sequencer URL is accepted by this local settlement driver.

## Recheck the recorded member workflow

From `lifecycle/`, verify the saved October 13 receipts in a fresh process. This consumes the exact recorded instructions; generating a new demo would create different proof inputs. Install the pinned `r0vm` with `client/scripts/install_r0vm.py` if needed, then supply its absolute path below. This source package uses the rebuilt `target/release/lifecycle-host`. The repository-level `scripts/review.py receipts` also runs saved-receipt verification directly without the measured runner's executor prerequisite.

```sh
python3 scripts/run.py check-succinct --state-profile standalone \
  --host "$PWD/target/release/lifecycle-host" --r0vm /absolute/path/to/r0vm \
  --create-instruction "$PWD/client/reports/2026-10-13-member-workflow/create-export/instruction.borsh" \
  --transfer-instruction "$PWD/client/reports/2026-10-13-member-workflow/spend-export/instruction.borsh" \
  --report-dir reports/2026-10-13-client-real-lifecycle --trial recheck
```

Choose a new trial name on subsequent checks. To repeat actual node inclusion, first build the unchanged pinned standalone `sequencer_service` as described in `../REAL_WORKFLOW.md`. The settlement driver starts a fresh child node, submits only the three saved test transactions, checks the node's commitment witnesses, and shuts the node down.

```sh
python3 scripts/settle_local.py \
  --sequencer "$PWD/target/release/sequencer_service" \
  --host "$PWD/target/release/lifecycle-host" --r0vm /absolute/path/to/r0vm \
  --upstream ../../lez-upstream \
  --bootstrap-dir reports/2026-10-12-standalone-funding \
  --proof-dir reports/2026-10-13-client-real-lifecycle \
  --report-dir reports/rechecked-client-settlement \
  --create-instruction "$PWD/client/reports/2026-10-13-member-workflow/create-export/instruction.borsh" \
  --transfer-instruction "$PWD/client/reports/2026-10-13-member-workflow/spend-export/instruction.borsh"
```

The historical native-funding receipt is reused because its genesis, private funder and amount are unchanged. The new member-driven creation and payment receipts have their own inputs and signatures. `resume-provenance.json` records reuse of two verified creation approval checkpoints from the earlier trial of this same member workflow. This run is not an uninterrupted fresh end-to-end benchmark.

After the saved-receipt check and settlement pass, correlate the exact client instruction hashes, current response verification, proof reports and node observations:

```sh
python3 client/scripts/verify_workflow.py \
  --client "$PWD/target/release/treasury-client" \
  --demo-dir client/reports/2026-10-13-member-workflow \
  --execution-dir reports/2026-10-13-client-guest-execution \
  --proof-dir reports/2026-10-13-client-real-lifecycle \
  --settlement-dir reports/rechecked-client-settlement \
  --output reports/rechecked-member-workflow.json
```

Choose new settlement and output paths for another replay. The correlation script checks that the reports describe the same workflow; the host's independent receipt check provides cryptographic verification. Export reports and the original demo transcript describe their creation time, before proving and submission. The later `verification.json` records the completed external proof and settlement checks.

## Rust API

| Call | Result |
| --- | --- |
| `Context::from_descriptor`, `Context::parts`, `Context::id` | Validate and identify the local configuration reference |
| `unsigned_create`, `unsigned_transfer` | Construct unsigned canonical requests |
| `decode`, `review` | Decode actual guest bytes and compute the review/signing digests |
| `respond` | Sign one reviewed approval or rejection |
| `verify_response`, `collect` | Verify and deduplicate asynchronous responses |
| `status` | Recompute threshold progress and rejection attribution |
| `export` | Verify quorum and produce the guest-compatible instruction |

There are no GUI, RPC, or transport dependencies in these operations. A future Logos core module can wrap this API; the module packaging and Logos Chat adapter have not been implemented.

## Milestone coverage

This advances the proposal lifecycle and byte-derived review. It does not complete the RFP: arbitrary actions and the full registry, configuration changes, role separation, time locks, spending limits, encrypted coordination, private-account/key sharing, disclosure/public postures, Basecamp QML, SPEL IDL, catalogs, public devnet compatibility, largest viable threshold measurement, and independent audit remain open.

Read `CLIENT_RESULTS_2026-10-13.md` for the dated run and its actual verification level. Source code for a mode is not evidence that the mode passed.
