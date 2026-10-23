# Trust model

## Approval and collection

The guest enforces the supported treasury rules. Members sign canonical bytes,
and the client derives its displayed action from those bytes. A collector can
withhold messages and present different partial collections. It cannot create
eligible signatures, count a duplicate twice, change a signed amount or relabel
a signed rejection as an approval. Rejections are opinions rather than vetoes;
five approvals may satisfy a five-of-seven treasury alongside two rejections.

The offline `review` and `respond-offline` commands accept caller-supplied
context. They are useful for controlled fixtures and do not certify accepted
chain state. The low-level Rust `respond` API has the same caller-context limit.

## State before signing

`import-state`, `review-verified` and `respond-verified` replay receipt-bearing
transactions from a base state and compare the complete final state hash with a
separately supplied trusted checkpoint. The complete hash includes nullifiers;
commitment inclusion alone cannot show that a note remains unspent.

The verifier checks real receipts, signature/nonce/window rules, note creation,
the local nullifier key, and whether the candidate's update nullifier appears
before the checkpoint. Verified review binds the checkpoint and account
commitment; verified signing rechecks them and checkpoint freshness.

| Assumption | What the current client checks | What remains external |
| --- | --- | --- |
| Checkpoint belongs to the intended chain | Domain, vault, guest identity and exact base/head state hashes match | Consensus/finality authentication and selection of an independently trusted checkpoint |
| Note is unspent | No candidate update nullifier through the verified checkpoint | Changes after that checkpoint; execution still needs normal sequencer validation |
| State is fresh enough | Five-minute maximum age, local-clock bounds, exclusive expiry and minimum counter | Reliable local clock and persistent latest-checkpoint/rollback policy |
| Member understands the action | Review is decoded from the actual signed bytes | Human review, supported decoder coverage and production signing UI |
| Local keys are protected | Restrictive file permissions and symlink checks for development key files | Encrypted keystore, hardware wallets and secure group-key distribution |

The initial evidence format is limited to 16 MiB and 128 private transactions,
and the base must precede creation of the candidate note. It is not a full-chain
indexer. Identical cloned chains need an external chain anchor; an application
domain string alone does not distinguish them.

The fixture generator reconstructs complete historical local state, including
the configured public genesis funding account. Its short-lived checkpoint is
test data, not evidence of current network finality. Keys already published as
deterministic fixtures remain publicly known.
