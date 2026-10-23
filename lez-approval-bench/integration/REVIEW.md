# Scope and acceptance review

This prototype addresses one operation: a native-token transfer approved by M distinct configured signing keys, funded by a preinitialized private PDA vault. It does not complete the full RFP or a production treasury product.

| Requirement | Implementation and evidence | Remaining work |
|---|---|---|
| Current member list and threshold | `VaultState` comes from the committed private configuration shard. The exact config digest is signed. Native and guest checks reject a changed threshold or current member ordering. | Creation, enrollment, rotation and reconfiguration workflow. |
| Distinct attributed approvals | BIP-340 signatures, key validation, unique member keys and unique member indices. Fixture keys map to upstream LEZ public-account IDs. | Wallet UX and proof of ownership during actual member enrollment; private member-account flows. |
| Exact-action binding | Network domain, vault, executing program, target program, epoch, proposal counter, expiry, recipient, custody seed and amount are bound into the signed action. | A network-domain assignment tied to the intended deployment. |
| Private custody | Private PDA authority is the shadow program's image-derived address. The native-token call receives its custody seed only in the plan whose approval effect must succeed. | Vault initialization/funding and recovery/migration UX. |
| Replay protection | Approval application advances an authoritative proposal counter. LEE independently advances private nonces and enforces commitment/nullifier rules. | Multiple sequential transactions, concurrent proposals and recovery under reorgs. |
| Expiry | The guest emits an exclusive timestamp upper bound in LEZ milliseconds. The complete proof binds that window; LEE settlement evaluates it. | Live sequencer timestamp/clock integration and user-facing deadline handling. |
| Private execution | The benchmark uses a shadow program and two private accounts. The intended publication uses a succinct PPE receipt; intermediate/composite receipts are local diagnostics that can reveal proof structure. | Wallet padding, transport privacy, network metadata review, shared viewing/nullifier key lifecycle. |
| Performance | Execute-only measurements for 1/7, 3/7 and 5/7; a separate real-proof run for 5/7. | Actual devnet limits, RPC/sequencer acceptance, maximum viable M, repeated runs and GPU comparison. |
| Security review | Positive and 19 negative native/guest checks, with settlement checks in the real-proof path. | Independent audit and remediation before deployment with user funds. |

The full-proof host uses upstream's execution engine, native-token code, private witness types, framing, Borsh encodings, state transition and pinned PPE ELF. Its measured proving backend is implemented locally to set bounded segment sizes and keep stage evidence. Upstream sources are unchanged. The root program is restricted to the single benchmark shadow program; it is not a general wallet execution service.

The committed debug/devnet block-size comparison and hand-assembled outer enum tag must not be presented as live network acceptance. A local `V03State` transition does not exercise the common crate's sequencer-level system-account guards, mempool admission, fees, block packing or Bedrock inscription. This fixture has no public account writes, and no production keys or assets.

Single-machine CPU timings include the host's receipt verification and evidence handling. Lightweight development/report tasks ran concurrently. The trial is a feasibility observation, not a statistically controlled performance claim. Receipt caches are verified on reuse; cached runs must not be reported as fresh proving times.
