# Vault-state import completed — 17 October 2026

The Rust client now imports a private vault note relative to a separately trusted
base/head checkpoint. It replays each receipt-bearing transaction through the
pinned LEE state machine, checks the resulting complete state hash, derives and
checks the candidate's spend nullifier, and reconstructs the signing context from
the committed treasury shard. Review and signing also bind the checkpoint and
account commitment, and recheck checkpoint freshness.

Local validation: 12 new state-import checks, 13 existing client workflow tests,
and 10 shared approval-core tests passed (35 Rust tests). Six fresh offline CLI
negative controls passed. The optimized Linux client and the recorded-fixture
example built successfully. Formatting checks passed for changed client Rust code.

The current-note control uses the saved 13 October native funding, creation and
payment transactions. Their real receipts are verified during replay. It imports
counter 1 and balance 463. The stale-note control confirms the older creation note
still has a commitment membership proof, then rejects it because its update
nullifier is present in the later verified transaction. Other checks cover altered
accounts, wrong keys/anchors, modified proof-bound messages, omitted/reordered
transactions, fake receipts, time/counter bounds and actual CLI review/signing.

The positive-control fixture includes the recorded standalone node's explicit
1,000,000-unit public genesis allocation. The earlier builtins-only export omits
that configured allocation, and the old private-workflow snapshots omit its
unrelated post-funding public shard. The fixture reconstructs both complete state
views consistently, including the 1,000-unit debit, before hashing them. Replay
validity timestamps are local test values, not new observations of block times.

No fresh proof generation, new node inclusion, public-network interaction or
hosted CI run was performed for this milestone. The older 13 October proof and
standalone-node evidence is retained with its original provenance. The approval
core, lifecycle program core, guest source and guest image remain byte-equal.

The imported state is authenticated relative to the caller's independently
trusted checkpoint. The example makes a short-lived historical-test checkpoint
from known archived data; it does not authenticate a public network. A live
checkpoint source, automatic wallet synchronization, private-account key
distribution, encrypted coordination and remaining RFP features remain open.

The evidence format is bounded to 16 MiB and 128 private transactions, and the
base must precede creation of the candidate note. This is not a scalable wallet
indexer. Unspent status is established at the checkpoint; later concurrent state
changes still require normal guest/sequencer validation at execution.

Read client/STATE_IMPORT.md and reports/2026-10-17-state-import/validation-summary.json.
The old ambiguous CLI `respond` refuses to sign; use `respond-verified` for the
checkpoint path or explicitly choose `respond-offline` for the local fixture demo.
