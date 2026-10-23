# Reproduce the evidence

Use an isolated checkout and public local fixtures. These instructions do not
deploy to a public network. The pinned historical LEZ source is fetched into
`lez-upstream/` beside `lez-approval-bench/` inside this repository.

## 1. Inspect without a Rust toolchain

```sh
python3 scripts/verify_evidence.py
```

This checks hashes and correlations, not RISC Zero proofs. Final receipts are
included; large intermediate proving checkpoints and precompiled host/client
binaries are omitted. `docs/provenance/omitted-from-full-archive.txt` lists them.

## 2. Build and run the Rust tests

Prerequisites: Linux x86_64, Git, Python 3.11+, a C/C++ build environment,
`pkg-config`, PC/SC development headers, OpenSSL development headers and Rustup.
The historical host toolchain is Rust 1.98.1. The full dependency build can require
several GiB of storage and memory. The wrapper uses two build jobs.

```sh
python3 scripts/review.py setup
python3 scripts/review.py test
```

`setup` fetches the exact official LEZ revision, checks that it is clean, and
installs the host toolchain through the existing Rustup installation. `test`
uses `--locked` and the included guest image, whose hash the build script checks.
It runs the 25 client tests and 10 shared approval-core tests. It does not rebuild
the guest or generate a new proof. Fresh test runs are required before describing
the workflow as green on another environment.

Equivalent manual build, from `lez-approval-bench/lifecycle/`:

```sh
LIFECYCLE_PREBUILT_GUEST="$PWD/reports/lifecycle/lifecycle-guest.bin" \
  cargo +1.98.1 build --release --locked -p treasury-client -p lifecycle-host
LIFECYCLE_PREBUILT_GUEST="$PWD/reports/lifecycle/lifecycle-guest.bin" \
  cargo +1.98.1 test --release --locked -p treasury-client
```

The guest source and its lockfile are retained. Rebuilding that guest requires
the historical RISC Zero guest toolchain; an arbitrary host compiler is not a
byte-reproducible replacement. The verification path above intentionally uses
the exact guest that the historical receipts bind.

## 3. Verify the saved real receipts in a fresh process

After the build:

```sh
python3 scripts/review.py receipts
```

The wrapper copies the relevant historical evidence to a new directory under
`var/`, supplies the exact recorded member instructions, and runs the rebuilt
host's `check-succinct` and `check-bootstrap` modes with development mode disabled.
The host checks real receipts, message/receipt binding, state transitions and
negative controls. It creates new check records without overwriting historical
reports. It does not need to generate new receipts or submit a transaction.

## 4. Exercise fresh local signers

```sh
python3 scripts/review.py demo
```

Seven new test keys are created in a temporary directory outside the checkout.
The public report goes under `var/`. The script removes the temporary test keys
when the run ends and retains only reports. It tests offline collection and six
negative CLI controls. This demo is explicitly offline and does not authenticate
live state. The checkpoint test suite from step 2 exercises verified signing.

## 5. Fresh historical checkpoint fixture

From `lez-approval-bench/lifecycle/`, choose fresh paths, with the test key outside
the checkout:

```sh
LIFECYCLE_PREBUILT_GUEST="$PWD/reports/lifecycle/lifecycle-guest.bin" \
  cargo +1.98.1 run --release --locked -p treasury-client \
  --example state_import_fixture -- /tmp/treasury-state-demo /tmp/treasury-nullifier-test.key
```

The checkpoint lasts only briefly; do not reuse the expired checkpoint bundled
in older reports. This example makes a local checkpoint from archived fixtures.
It does not establish current chain state or finality. Remove the temporary test
key after inspecting the fixture.

## 6. Fresh proving and standalone-node replay

The detailed historical recipe is in
`lez-approval-bench/lifecycle/REAL_WORKFLOW.md`, with the member-specific arguments
in `lez-approval-bench/lifecycle/client/README.md`. Build the pinned official
`sequencer_service` with `--features standalone`, use the exact historical guest,
and select fresh output paths. `scripts/settle_local.py` starts a loopback-only
child node, submits the three saved fixture transactions, checks their bytes
and returned membership witnesses, and shuts it down.

The historical README's `../../bin/*` paths refer to the original full archive.
In this source release, use `target/release/treasury-client` and
`target/release/lifecycle-host` from the lifecycle workspace instead. The
sequencer binary must be copied or passed explicitly from the pinned upstream build.

Actual guest execution and fresh proving use the exact official CPU `r0vm`:

```sh
python3 lez-approval-bench/lifecycle/client/scripts/install_r0vm.py \
  --output /tmp/treasury-tools/r0vm
```

That installer verifies the official archive and binary digests. Full proving is
a substantial optional workload. This release's checks do not imply that it was
rerun, nor that local replay establishes testnet 0.4 compatibility.
