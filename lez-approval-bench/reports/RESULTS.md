# Verified 5-of-7 approval proof

Recorded 7 October 2026, after submission of Logos RFP-005 proposal #211.

**A real RISC Zero composite receipt was produced and verified.** Five distinct BIP-340 approvals in a seven-member configuration were checked by the standalone guest. A separate process verified the saved receipt against the retained guest program and expected journal. Development mode was disabled and fake receipts were explicitly rejected.

This establishes the standalone approval-proof result. Full LEZ private-account integration and the RFP P.1/P.2 transaction benchmarks remain open.

## Measured result

| Measurement | Result |
| --- | ---: |
| Required approvals / members | 5 / 7 |
| Proving wall time | 642.289 seconds (10 min 42 sec) |
| Receipt verification in the proving host | 190.412 ms |
| Saved composite receipt, bincode 1.3.3 | 2,268,522 bytes (2.16 MiB) |
| Guest user cycles | 1,691,367 |
| Total proof cycles, including overhead and padding | 2,162,688 |
| Paging / reserved cycles | 203,394 / 267,927 |
| Segments | 9 |
| Segment limit | 2^18 cycles |
| Largest child process peak RSS | 2,423,740 KiB (2.31 GiB) |
| Public journal | 288 bytes |
| Serialized guest input | 3,060 bytes |
| Exact guest program | 356,252 bytes |

The run began at 2026-10-07T11:43:28.308758+00:00. Its measurement wrapper completed with exit code 0 after 642.507 seconds. Proving time includes guest execution and receipt generation, and excludes compilation and the later verification checks. Peak memory is Linux RUSAGE_CHILDREN's largest process peak, not simultaneous aggregate memory.

Hardware: Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; 9 visible logical CPUs; cgroup quota of 8 CPU equivalents; 8 GiB memory limit; 8 Rayon workers. No GPU, paid prover, network transaction, wallet, or transferred assets were used. This is one observation on a shared host, not a median or production latency claim. Dependency metadata and documentation work occurred during the run; no separate proof workload was launched.

## Change from the baseline

The guest now applies the official RISC Zero acceleration patches for k256 0.13.4, sha2 0.10.9 and crypto-bigint 0.5.5, each pinned to an exact Git commit. Its release profile uses one code generation unit and thin LTO. The approval rules in `core/src/lib.rs` are unchanged. The host retains upstream registry crates as the independent fixture control. Exact dependencies and commits are in both lockfiles and `sources.json`.

The same 5-of-7 fixture decreased from 30,197,085 to 1,691,367 user cycles, a 94.40% reduction, and from 141 to 9 segments. This is a cycle comparison. The earlier proof was interrupted, so no completed proving-time speedup is claimed. Its four-worker configuration also differs from this run's eight-worker configuration.

The original execution and interrupted-proof records remain in `reports/`; `BASELINE-RESULTS.md` preserves that earlier report. `baseline-source/` retains the prior manifests, lockfiles, host, verifier and harness sources.

## Checks completed

- All 10 native tests passed, including the 15 published 32-byte-message BIP-340 vectors.
- A separate accelerated validation guest passed 21 approval cases against the unpatched host, including quorum, duplicate approvals, wrong signer, malformed input, action changes and ordering. It also checked all 15 published vectors inside the accelerated zkVM. This validation run executed without proving and is labelled accordingly.
- The proof host verified the real receipt and exact expected 5-of-7 journal.
- A fresh verifier process read and verified the saved receipt against `accelerated/approval-guest.bin`.
- A wrong expected threshold and the baseline guest image were both rejected.
- Modifying the decoded journal caused cryptographic verification to fail. The original receipt file's SHA-256 was unchanged. These checks use the structured receipt API; an initial byte-pattern approach refused an ambiguous match and its log is retained as `initial-byte-offset-check.log`.

## Evidence files

All current-run files below are under `reports/accelerated/`:

- `m5-n7.receipt`, `m5-n7.json`, `m5-n7.resources.json`: proof, measurements and machine/resource record.
- `approval-guest.bin`: exact proved program; SHA-256 `69ccf93cdcfdabd15df11fbabb18dbd6488859778ec1a97e15488737402f1ff8`.
- `saved-receipt-verification.json`, `receipt-negative-checks.json`: fresh-process verification and integrity controls.
- `backend.validation.json`, `approval-validation.bin`: separate accelerated backend validation, including its image ID.
- `m5-n7.execution.json`: execution-only diagnostic, explicitly not proof evidence.
- Build, unit-test, execution and proving logs; resolved guest dependency tree and metadata.

Receipt SHA-256: `9520f6321d04a3afd0682ac78d9b2d2fd498a682f9e0da090842899951984d9a`. The project-wide `SHA256SUMS` covers source and retained evidence.

## Remaining integration work

The guest receives its configuration as input. The eventual LEZ program must establish authoritative account ownership and current configuration, enforce expiry and replay protection, and bind the proved action to the instruction actually executed. The public journal exposes the threshold, member count and signer indices. Private custody, membership confidentiality, revocation, coordination and selective disclosure are not implemented by this verifier.

The 2.16 MiB composite receipt is a local serialization, not a LEZ transaction. The next gate is to integrate with a pinned LEZ private-account execution path and measure complete transactions for M = 1, 3 and 5 against actual network limits and the required receipt encoding. This result does not complete RFP P.1/P.2 or the $8,000 M1 milestone by itself.

The prototype is unaudited and uses public deterministic fixture keys. The submitted proposal remains available at https://github.com/logos-co/rfp/issues/211; these later results are retained for a future reviewer update.
