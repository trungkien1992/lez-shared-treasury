# Private LEZ vault integration

This bounded benchmark executes an M-of-7 native-token transfer from a private program-derived vault to a fresh private recipient. It links the unmodified LEE state machine at commit `411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4` and uses that commit's prebuilt privacy circuit. The member list and threshold are read from the vault's committed private account state.

A real 5-of-7 transaction proof and succinct receipt completed in 46.99 minutes. The 232,072-byte envelope fits the pinned repository's configured cap. A fresh process verified and applied the saved transaction, rejecting expiry, replay and message tampering. Read `reports/RESULTS.md` for the measured results and boundaries.

The execution measurements are in `reports/m1-n7/execute.json`, `reports/m3-n7/execute.json`, and `reports/m5-n7/execute.json`. All three runs decrypt the output notes and check the exact resulting accounts. `reports/m5-n7/validation.json` records one positive native/guest comparison and 19 invalid cases rejected by both implementations. Real complete-proof results are in `reports/m5-n7/succinct.json` and its receipt/transaction files. Execution-only results are never evidence of a completed proof.

## What the program does

The program uses a shadow-program address derived by LEZ from the exact guest image. The private vault is a PDA bound to that address and its custody seed. The vault holds a configuration shard and a native-balance shard. This benchmark seeds a funded vault in an in-memory genesis state; it does not provide a vault-creation or member-enrollment workflow.

A transfer contains the configuration epoch, next proposal number, expiry in Unix milliseconds, custody seed, recipient and amount. Each BIP-340 signature binds the current configuration digest, network domain, vault, program, target native-token program and that exact transfer. The shared approval verifier rejects invalid keys, duplicate members, nonmembers, repeated approvers and insufficient approvals.

LEZ first executes the program's approval effect against the private configuration shard. Successful verification increments the proposal counter. Only then does its execution engine dispatch the planned native-token custody transfer. A failure aborts the complete transaction. The privacy circuit binds the vault state to a membership proof, updates private nonces/nullifiers, and encrypts the resulting accounts. Settlement checks the proof, commitment roots, freshness, nullifiers and the proof-bound expiry window.

Member fixture signing keys map to LEZ public-account identities using upstream `PublicKey` and `AccountId` conversion. No member-account enrollment, private-member ownership handshake or rotation protocol is implemented. The member signing keys and the private vault's viewing/nullifier material serve separate roles. Fixture keys and randomness are public and deterministic; this package is for local testing only.

## Measurement boundaries

- The private accounts and shadow-program identity use actual pinned LEE code. No fake receipts or development proving mode are accepted.
- The instrumented host mirrors the pinned prover using public APIs. It adds 2^18-cycle segments for memory control, timings and verified stage checkpoints. It does not patch the state machine or PPE circuit.
- A real proof first creates a full composite PPE receipt, then compresses it to the succinct format used by upstream's proving helper. A failed compression leaves the verified composite checkpoint available.
- Stage plan/apply receipts and logs are prover-side debugging artifacts containing private fixture inputs. The composite PPE checkpoint can expose component proof structure and image claims. Use the final succinct PPE receipt and its encoded transaction as the intended publication artifacts; the composite form is diagnostic.
- This case uses two real notes, no dummy notes and 1024-byte ciphertext padding. It does not reproduce the wallet's seven-note padding policy. Published ciphertext sizes and timing metadata still need a full privacy assessment.
- `transaction-*.borsh` is the actual upstream `PrivacyPreservingTransaction` Borsh encoding. `transaction-*.core` is its core encoding. The `.lee-envelope` file adds the one-byte `common::LeeTransaction` variant tag from the pinned source. The common crate, network RPC, sequencer and Bedrock are not exercised.
- The 1 MiB block and 2048-byte overhead comparison comes from both the repository's debug and committed Docker devnet configurations. It is not a verified live devnet limit or throughput result. Private transactions are fee-exempt in this pinned version; this benchmark is not a cost quotation.

## Reproduce

Use the Rust/RISC Zero versions in the parent README. Keep this project at `lez-approval-bench/integration` and clone the upstream repository beside `lez-approval-bench`:

```sh
git clone https://github.com/logos-blockchain/logos-execution-zone.git lez-upstream
git -C lez-upstream checkout --detach 411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4
cd lez-approval-bench/integration
CARGO_BUILD_JOBS=2 RISC0_BUILD_LOCKED=1 cargo build --release --locked -p vault-host
python3 scripts/run.py execute --m 1 --trial reproduction --r0vm /absolute/path/to/r0vm
python3 scripts/run.py execute --m 3 --trial reproduction --r0vm /absolute/path/to/r0vm
python3 scripts/run.py execute --m 5 --trial reproduction --r0vm /absolute/path/to/r0vm
python3 scripts/run.py validate --m 5 --trial reproduction --r0vm /absolute/path/to/r0vm
```

The runner refuses to overwrite a trial's log or resource record. Real stages can resume only after input-hash and receipt verification. Use `--report-dir reports/reproduction/m5-n7` for a fresh proving benchmark. Omitting that option resumes the delivered directory and may reuse verified stages; its invocation wall time is not a fresh proving-time measurement. Keep the delivered evidence intact.

```sh
python3 scripts/run.py succinct --m 5 --report-dir reports/reproduction/m5-n7 --trial reproduction --timeout 5400 --r0vm /absolute/path/to/r0vm
python3 scripts/run.py check-succinct --m 5 --report-dir reports/reproduction/m5-n7 --trial reproduction --r0vm /absolute/path/to/r0vm
```

The fresh check loads the saved receipt and transaction, verifies the pinned circuit ID, decrypts the expected output notes, applies the transaction to the original LEE state, and rejects expiry, replay and a changed public message. Failed transactions must leave state unchanged. Use `check` for the composite transaction if succinct compression is unavailable.

The first 1-of-7 and 3-of-7 execution attempts exposed a harness error: commitments and note/nullifier pairs are sorted independently by LEZ. Trial 2 fixes note matching by nullifier and checks commitments independently. Those failed trial logs remain as evidence; they were not failed LEZ transfers.
