# Creation and enrollment results

Recorded 7 October 2026. The new guest executes both creation/funding and a subsequent 5-of-7 transfer through pinned LEZ. **These are execution-only results; no new receipt was generated or transaction settled.** The earlier real proof remains tied to the unchanged v1 guest in `../../integration/`.

## State observed

| Step | Funding account | Vault | Recipient | Vault proposal counter |
| --- | ---: | ---: | ---: | ---: |
| Initial fixture | 1,000 | Absent | Absent | Absent |
| Create and fund | 500 | 500 | Absent | 0 |
| Spend 37 | 500 | 463 | 37 | 1 |

Only the funding account exists in the initial in-memory genesis fixture. Seven keys sign the descriptor for a five-signature spending threshold. The first transaction creates the private vault configuration from an empty shard and funds it using the independent account's authorization. The host decrypts both notes and checks exact account data, nonces, kind, commitments and nullifiers. It explicitly inserts the checked candidate commitments into simulated state to build the second execution's membership witness. The spend execution produces another two exact checked notes. This intermediate insertion is not proof-verified settlement.

## Actual RISC-V user cycles

| Stage | Create/fund | Spend |
| --- | ---: | ---: |
| Plan | 58,597 | 25,691 |
| Apply | 2,196,804 | 1,618,281 |
| PPE execution | 2,084,856 | 2,052,733 |
| Sum | 4,340,257 | 3,696,705 |

The complete execute invocation took 1.171 seconds; maximum child RSS was 51,500 KiB on the recorded 8-CPU/8-GiB environment. These are dry-execution observations and must not be quoted as proving latency, network throughput or transaction size. RISC-V user cycles exclude Keccak accelerator/recursion proof work. Each transaction uses two real notes, zero dummy notes and 1024-byte ciphertext padding.

## Validation

- 40 invalid Apply cases rejected by both native and accelerated guest execution, including missing/duplicate enrollment, a changed descriptor with a rebound address, malformed configuration/key material, overwrite attempts, changed spending actions, replayed counters and enrollment signatures used for a spend.
- Five invalid Plan cases rejected by native and guest: unauthorized funding, incorrect vault/funding handles, zero funding amount and extra handles on unfunded creation.
- Two complete funding executions reject a missing source authorization key or insufficient source balance and return no PPE output.
- Valid native/guest creation and transfer results match; unfunded creation writes the same enrolled configuration; the exclusive expiry window reaches the PPE output.
- Six local HTTP/preflight tests pass, including an incorrect channel and refusal to call a write method.

Validation trial 1 ended with a harness error after the program correctly rejected insufficient funding: the check expected the word “insufficient”, while upstream displayed “holds less than the transferred amount”. Trial 2 matches the typed `TransferError::InsufficientBalance` and passes. Both logs and resource records are retained. No rejection caused by an unrelated executor/infrastructure error is counted as success.

## Exact artifacts

Guest SHA-256: `fff9878b2215978e6be4bd62ac1f89a4c3b1b351bf4f0d61b308eaa0c30e321c`. Its image ID and pinned PPE hash appear in both `lifecycle/execute.json` and `validation/validate.json`. The exact guest is saved in each result directory. Canonical fixture bytes are `lifecycle/enrolled-descriptor.borsh` and `lifecycle/create-instruction.borsh`; checked circuit outputs are saved separately. All fixture keys are public and deterministic.

A byte comparison against the previous delivered archive confirmed all 549 existing integration files were unchanged, including v1 source, guest binaries and real receipts. External host and guest package versions/sources match the earlier integration lockfiles; only local package names changed.

## Remaining gates

Generate real creation and spend proofs for this guest and validate actual state transitions; ingest live private witnesses and settle on a confirmed LEZ test deployment; implement encrypted invitations, private identity/key distribution, rotation and the rest of the product requirements; commission an independent audit. The offline devnet preflight passes source/PPE identity checks and records missing endpoint/channel/runtime evidence. The current environment has no Docker or `just` executable. No new estimate of mainnet safety, live capacity or grant completion is made.
