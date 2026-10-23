# Verified private LEZ transfer — 5 of 7

A complete private transfer was proved with real RISC Zero receipts and accepted by the unmodified pinned LEE state machine. The succinct receipt and encoded transaction also passed verification in a separate process. The public issue remains the original approved submission; this is subsequent engineering evidence.

The complete workflow took **2819.19 seconds (46.99 minutes)** on the recorded 8-vCPU, 8 GiB, CPU-only environment. The final LEZ-format envelope is **232,072 bytes (226.63 KiB)**. The observed peak child-process RSS was **3,637,160 KiB (3.47 GiB)**. This is a single local feasibility run; lightweight development/report tasks shared the machine.

## Real proof stages

| Stage | Wall seconds | RISC-V user cycles |
|---|---:|---:|
| vault-plan | 20.29 | 21,847 |
| vault-apply | 642.26 | 1,604,221 |
| ppe-composite | 1035.09 | 2,051,313 |
| Succinct compression | 1118.94 | Separate recursion work |

The real RISC-V proof stages total 3,677,381 user cycles. These counts do not represent all Keccak accelerator and recursion proving work. Stage timing includes receipt verification and evidence handling. The total workflow also includes state-validation and encoding checks.

## Transaction size

| Encoding | Composite diagnostic | Final succinct |
|---|---:|---:|
| Borsh InnerReceipt | 5,312,145 bytes | 227,601 bytes |
| Upstream PrivacyPreservingTransaction | 5,316,615 bytes | 232,071 bytes |
| LEZ enum envelope | 5,316,616 bytes | 232,072 bytes |

The pinned repository's debug and Docker devnet configurations specify a 1 MiB block. RPC admission subtracts 2048 bytes, yielding a configured single-transaction cap of 1,046,528 bytes. The composite form exceeds that cap. The succinct form fits it. This is a source-based comparison, not live devnet acceptance, a throughput result or confirmation of a running network's current configuration.

The envelope is the pinned `common::LeeTransaction` layout: Borsh tag 1 followed by the actual upstream private-transaction encoding. The common crate/RPC/sequencer/Bedrock were not run. Composite/component receipts remain diagnostic because they can reveal proof structure; the intended published form is the succinct transaction.

## Verified behavior

- The private PDA vault reads its 5-of-7 configuration from committed account state. There are no public account effects or outer public signatures.
- The vault balance moves from 1000 to 963; the fresh private recipient receives 37; the proposal counter advances from 0 to 1.
- Both encrypted output notes decrypt to the exact expected account states. Commitment presence is checked independently because LEZ shuffles commitments separately from nullifier/note pairs.
- One valid approval apply matches native Rust and the accelerated guest. All 19 invalid cases are rejected by both.
- In a fresh process, LEE accepts the saved transaction, rejects it at the exact expiry boundary, rejects replay, and rejects a modified proof-bound message. Each failed transition preserves state. The replay is rejected at the commitment-freshness check (`Commitment already seen`); this trial does not isolate the subsequent nullifier check.
- Both upstream transaction encodings round-trip. The real receipt verifies against the pinned PPE image ID. Development mode and fake receipts are forbidden.

## Scaling measurements

| Threshold | Plan cycles | Approval apply cycles | PPE cycles | Total user cycles |
|---|---:|---:|---:|---:|
| 1 of 7 | 16,745 | 500,467 | 2,043,896 | 2,561,108 |
| 3 of 7 | 19,281 | 1,052,996 | 2,047,437 | 3,119,714 |
| 5 of 7 | 21,847 | 1,604,221 | 2,051,285 | 3,677,353 |

All three rows are complete **execution-only** measurements. Real proving was performed only for 5 of 7. Its PPE proof reports 28 more user cycles than the corresponding execution-only trial; the respective raw measurements are retained. These observations do not determine the largest viable M.

## Scope and next validation

The fixture uses a preinitialized private vault, public deterministic test keys, a synthetic network domain, two real notes, no dummy notes and 1024-byte ciphertext padding. It does not implement enrollment, initialization/funding, reconfiguration, wallet coordination, seven-note padding, production secret management or the complete RFP. No independent audit, deployment or live transaction submission has occurred.

Next, integrate vault creation and member enrollment, validate the final envelope through the actual sequencer/devnet, confirm current limits with the maintainers, and benchmark the intended wallet padding and larger thresholds. Assess GPU proving before promising interactive performance. Independent review and the proposed audit remain required before deployment with user funds.

## Evidence and reproduction

Read `../README.md`, `../REVIEW.md`, and `../sources.json`. Key files are `m5-n7/succinct.json`, `m5-n7/succinct-trial-1.resources.json`, `m5-n7/ppe-succinct.receipt`, `m5-n7/transaction-succinct.borsh`, `m5-n7/transaction-succinct.lee-envelope`, and `m5-n7/fresh-check-succinct.json`. The exact guest binary, lockfiles, stage receipts, failed initial note-matching trials and passing reruns are retained.

Pinned upstream: `411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4`. Relevant sources: `lee/state_machine/src/privacy_preserving_transaction/circuit/mod.rs`, `lee/state_machine/core/src/execution_state.rs`, `lee/privacy_preserving_circuit/src/output.rs`, `lez/common/src/transaction.rs`, `lez/configs/docker-all-in-one/devnet/sequencer_config.json`, and `lez/sequencer/actors/rpc_server/src/actor/service.rs`.
