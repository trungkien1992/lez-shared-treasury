# Real creation-to-spend workflow

This extension adds real proving and proof-verified state handoff to the archived
creation/enrollment guest. The guest remains byte-for-byte identical to the one
whose native and accelerated execution checks were recorded on 7 October.

The `execute` mode deliberately retains the old simulation for comparison. The
`prove` and `succinct` modes use `V03State::transition_from_privacy_preserving_transaction`
to accept the creation proof, then obtain the spend membership witness from that
accepted state. They do not insert candidate commitments manually. A spend witness
must be unavailable before creation, and the proved spend must be rejected against
the pre-creation state.

## Build the host around the exact tested guest

Keep `lez-upstream` next to `lez-approval-bench`, at the revision recorded in
`DEVNET.md`. Install host Rust 1.98.1 and RISC Zero/r0vm 3.0.5.

```sh
cd lez-approval-bench/lifecycle
LIFECYCLE_PREBUILT_GUEST="$PWD/reports/lifecycle/lifecycle-guest.bin" \
  CARGO_TARGET_DIR=../integration/target CARGO_BUILD_JOBS=2 \
  cargo build --release --locked -p lifecycle-host
```

The optional prebuilt path verifies SHA-256
`fff9878b2215978e6be4bd62ac1f89a4c3b1b351bf4f0d61b308eaa0c30e321c`
at build time. The host also computes the guest's actual image ID and records it
in every result. The default build, without this variable, still rebuilds the
guest with the pinned RISC Zero Rust toolchain. The recorded environment's guest
rebuild reached a host linker failure; the exact archived guest was used instead.

## Prove and verify in separate processes

Choose fresh report directories and a real r0vm executable. The runner disables
development mode and refuses to overwrite a prior trial. Completed proving stages
are checked before reuse; retain trial logs to distinguish fresh and resumed work.

```sh
python3 scripts/run.py validate --report-dir reports/my-validation --r0vm /absolute/path/to/r0vm
python3 scripts/run.py execute --report-dir reports/my-execution --r0vm /absolute/path/to/r0vm
python3 scripts/run.py succinct --report-dir reports/my-real-lifecycle --r0vm /absolute/path/to/r0vm
python3 scripts/run.py check-succinct --report-dir reports/my-real-lifecycle --r0vm /absolute/path/to/r0vm
```

`prove` retains composite proofs. `succinct` additionally compresses each full PPE
receipt into the format used for the local sequencer test. Each transaction is
checked for a valid state transition, exclusive expiry, message tampering, replay,
and preservation of state after rejection. All four private output notes are
decrypted and checked against exact expected account values and configuration.

The saved-proof modes verify receipt identity, journal-to-transaction binding,
transaction encodings, accepted state snapshots, and the creation-derived spend
witness. They do not generate new proofs. Each check mode writes its own result
with a new timestamp if an earlier result exists. Use a new runner `--trial` name
for repeated checks, and a copied proof directory for destructive-control experiments.

## Establish the same funding commitment through a real transaction

The original local fixture starts with one private funding commitment. A standalone
node should reach that state by processing a normal native-token transfer from a
public account funded in its test genesis, rather than by editing its database.

```sh
python3 scripts/run.py bootstrap-execute --report-dir reports/my-funding-execution --r0vm /absolute/path/to/r0vm
python3 scripts/run.py bootstrap --report-dir reports/my-native-funding --r0vm /absolute/path/to/r0vm
python3 scripts/run.py check-bootstrap --report-dir reports/my-native-funding --r0vm /absolute/path/to/r0vm
```

The funding proof debits 1,000 test units from a fixed public test account and
creates the private funding account. The host checks the note, the debit, replay
rejection, and equality of its accepted commitment root and membership witness
with the lifecycle fixture. This step does not create or fund the treasury itself.

The default `isolated` state profile reproduces the historical minimal fixture.
The pinned sequencer starts with two other private accounts and immutable program
mirrors, so its commitment tree is larger. The node workflow uses the `standalone`
profile and an exact genesis snapshot exported from the unchanged upstream
library. The snapshot's private root is compared with the running node before
using it. Approval receipts can be reused only after their input hashes match;
the PPE receipts and compression must be regenerated for the actual witnesses.
Reports distinguish reused checkpoints from fresh work. A resumed invocation's
wall time is not a new complete end-to-end proving benchmark.

## Standalone sequencer settlement

Build the pinned upstream service with its `standalone` feature:

```sh
cd ../../lez-upstream
CARGO_BUILD_JOBS=2 cargo build --release --locked --features standalone -p sequencer_service
```

Follow upstream's platform dependencies for this build. Its pinned Bedrock circuit
dependency requests the v0.5.7 Linux prebuilt artifact. If the dependency downloader
cannot connect, use the documented `LBC_ROOT_DIR` override with that exact official
artifact; keep its download metadata and hash. Do not patch the sequencer to bypass
validation, replace the pinned revision silently, or use development receipts.

Back in `lifecycle/`, settle the already verified transactions:

First export genesis and generate/verify reports using `--state-profile standalone`:

```sh
python3 scripts/export_genesis.py --upstream ../../lez-upstream \
  --target-dir ../../lez-upstream/target
python3 scripts/run.py succinct --state-profile standalone \
  --report-dir reports/my-real-lifecycle --r0vm /absolute/path/to/r0vm
python3 scripts/run.py check-succinct --state-profile standalone \
  --report-dir reports/my-real-lifecycle --r0vm /absolute/path/to/r0vm
python3 scripts/run.py bootstrap --state-profile standalone \
  --report-dir reports/my-native-funding --r0vm /absolute/path/to/r0vm
python3 scripts/run.py check-bootstrap --state-profile standalone \
  --report-dir reports/my-native-funding --r0vm /absolute/path/to/r0vm
```

```sh
python3 scripts/settle_local.py \
  --sequencer /absolute/path/to/sequencer_service \
  --host ../integration/target/release/lifecycle-host \
  --r0vm /absolute/path/to/r0vm \
  --upstream ../../lez-upstream \
  --bootstrap-dir reports/my-native-funding \
  --proof-dir reports/my-real-lifecycle \
  --report-dir reports/my-local-settlement
```

The script starts a fresh child node on loopback, disables gossip, verifies its
explicit channel and genesis funding state, and shuts it down afterward. It uses
the pinned `LeeTransaction` Borsh/Base64 JSON encoding documented in upstream
`lez/common/src/borsh_base64.rs`. It submits native funding, treasury creation, and
the 5-of-7 spend in order. Every accepted transaction must be retrievable with the
same bytes and a block ID. The node's funding root, creation membership witness,
and final spend root must match the independently verified proof workflow.
The host also ingests the node's returned creation membership proof, verifies its
root opening, and confirms that the resulting spend-witness bytes match the
already-proved input. This is a controlled fixture workflow, not a general wallet
adapter for arbitrary live accounts or unrelated network activity.

## Scope

All keys are public deterministic fixtures and all balances are local test units.
The application network field remains the explicitly synthetic fixture domain;
this is not an implemented wallet network-selection or key-sharing flow. The two
private notes per treasury operation use 1024-byte ciphertext padding and no dummy
notes. Larger thresholds, intended wallet padding, membership/key rotation,
selective disclosure, encrypted coordination, production interfaces, public devnet
compatibility, and independent review remain separate work.

Read the dated result JSON and `REAL_RESULTS.md` for what actually completed.
Source code for an available mode is not evidence that the mode passed.
