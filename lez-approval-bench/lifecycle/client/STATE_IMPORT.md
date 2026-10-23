# Checkpoint-backed vault-state import

This client can import a private vault note relative to an **independently trusted
local checkpoint**. It replays real receipt-bearing transactions from a pinned
base snapshot, checks the resulting complete state hash, and rejects the note if
its update nullifier occurs in that replay. The candidate note must have been
created after the base snapshot. A membership proof alone is insufficient because
spent commitments remain in the append-only tree.

This is a bounded local state-verification milestone. Checkpoint discovery,
network consensus/finality verification, automatic wallet synchronization, secure
distribution of private-account keys and Logos Chat remain separate work.

## Trust and freshness

`TrustedCheckpoint` is supplied through a separate caller-controlled path. Its
base/head hashes must come from state the operator independently accepts. Do not
copy a checkpoint from a proposer or treat an RPC response plus its own root as
authentication. The importer does not authenticate the checkpoint file itself.
The application network domain remains an application assertion; this mechanism
does not distinguish identical cloned chains without an external chain anchor.

The complete state hash includes spent nullifiers, unlike the commitment root.
Every replayed transaction passes the pinned LEE receipt, signature, nonce,
validity-window and state-transition checks. Fake receipts are explicitly rejected
and RISC Zero development-mode receipt acceptance is disabled.

A checkpoint must be observed no later than the local clock, be younger than five
minutes, and remain before its exclusive expiry. A caller-supplied minimum
proposal counter can additionally prevent rollback. The caller must maintain its
own latest accepted checkpoint/counter policy. There is no automatic persistent
checkpoint-selection or consensus service in this milestone.

Unspent status is established at the accepted checkpoint. Concurrent subsequent
activity can still make a transaction stale; the guest and sequencer remain the
execution authority. Reverification does not reserve funds.

## Commands

```sh
treasury-client import-state evidence.borsh trusted-checkpoint.json \
  /private/nullifier.key context.json

treasury-client propose context.json RECIPIENT_HEX AMOUNT_BASE_UNITS \
  EXPIRY_MS request.json

treasury-client review-verified evidence.borsh trusted-checkpoint.json \
  /private/nullifier.key request.json

treasury-client respond-verified evidence.borsh trusted-checkpoint.json \
  /private/nullifier.key request.json /private/member.key approve \
  REVIEW_DIGEST_FROM_VERIFIED_REVIEW response.json
```

`review-verified` displays the verified balance, counter, commitment and checkpoint.
Its review digest also binds the checkpoint and complete account commitment.
`respond-verified` reruns state verification, rechecks freshness and checks that
digest before signing. The resulting approval bytes and guest signature domain
remain compatible with the existing guest and response collector.

The imported `context.json` remains an ordinary data file. Its existence or a JSON
flag cannot create the library's opaque `VerifiedState` type. Keep using the
verified review/signing commands rather than interpreting the saved context as a
standalone certificate. The nullifier secret is read locally with restrictive
file permissions and is not saved into context, evidence, responses or reports.
It is separate from a member's approval-signing key.

The ambiguous CLI command `respond` now refuses to sign. The explicitly named
`respond-offline` preserves the earlier local-demo workflow. The low-level Rust
`respond` API retains its documented caller-supplied-context semantics; applications
requiring checkpoint verification must use `VerifiedState::respond`.

## Evidence format and boundaries

`StateEvidence` is canonical Borsh with a version, treasury descriptor, complete
candidate account, base-state bytes and ordered `AcceptedStep` records. Each record
contains the exact pinned LEE transaction Borsh bytes and replay block/time values.
The head checkpoint independently fixes the required result. Missing or altered
transactions cannot substitute an arbitrary state for that accepted result.

The first version supports at most 128 private-transaction records and a 16 MiB
evidence file. It needs a base preceding creation of the candidate note so it can
track that note's nullifier through the entire suffix without an upstream
non-membership-proof API. It is not a scalable full-chain wallet/indexer. Public
transaction replay, automatic checkpoint refresh and broader note discovery are
not implemented. The treasury guest still supports the existing creation/native
transfer subset, with no new configuration-change or spending-policy instructions.

## Validation and local fixture generation

Use the unchanged upstream revision
`411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4` beside `lez-approval-bench`, Rust 1.98.1,
the existing lockfile and archived guest:

```sh
cd lez-approval-bench/lifecycle
LIFECYCLE_PREBUILT_GUEST="$PWD/reports/lifecycle/lifecycle-guest.bin" \
  cargo test --locked -p treasury-client -- --nocapture
```

The checks use the saved real creation/payment proofs and accepted state snapshots.
They cover a current note, an older spent note with surviving membership, mutated
notes/keys/messages, checkpoint mismatches, incomplete replay, fake receipts,
freshness/counter bounds, checkpoint-bound signing and an actual CLI round trip.

For a reproducible **historical test fixture**, run the example with two fresh
paths. Keep the local test nullifier key outside the project:

```sh
LIFECYCLE_PREBUILT_GUEST="$PWD/reports/lifecycle/lifecycle-guest.bin" \
  cargo run --locked -p treasury-client --example state_import_fixture -- \
  /tmp/new-state-import-demo /tmp/new-test-nullifier.key
```

The example packages the archived 13 October post-payment state and makes a
short-lived local-demo checkpoint from the known archive snapshot. It does not
query a live node or establish public-network freshness. Its key material was
already public test-only material in the original fixture and must not hold funds.
The example prints paths and validation metadata, never key bytes.
