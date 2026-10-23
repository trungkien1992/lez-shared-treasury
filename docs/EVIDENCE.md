# Evidence index

Paths below are relative to the repository. Historical records retain their
original bytes, dates, commands and machine descriptions. Absolute paths in old
logs identify the original environment; use REPRODUCE.md for this checkout.

## 13 October — member workflow, real proofs and local inclusion

| Evidence | Path |
| --- | --- |
| Correlated member/proof/node summary | `lez-approval-bench/lifecycle/client/reports/2026-10-13-member-workflow/verification.json` |
| Public member keys, responses and exact exported instructions | `lez-approval-bench/lifecycle/client/reports/2026-10-13-member-workflow/` |
| Fresh-process receipt-check record | `lez-approval-bench/lifecycle/reports/2026-10-13-client-real-lifecycle/check-succinct.json` |
| Real final creation/payment receipts | `lez-approval-bench/lifecycle/reports/2026-10-13-client-real-lifecycle/{create,spend}/ppe-succinct.receipt` |
| Encoded creation/payment transactions | `lez-approval-bench/lifecycle/reports/2026-10-13-client-real-lifecycle/{create,spend}/transaction-succinct.lee-envelope` |
| Historical native-funding receipt and transaction | `lez-approval-bench/lifecycle/reports/2026-10-12-standalone-funding/` |
| Standalone-node observations and final balances | `lez-approval-bench/lifecycle/reports/2026-10-13-client-settlement/settlement.json` |
| Actual node membership matched to the proved spend input | `lez-approval-bench/lifecycle/reports/2026-10-13-client-real-lifecycle/check-node-witness.json` |
| Reused proof-stage provenance | `lez-approval-bench/lifecycle/reports/2026-10-13-client-real-lifecycle/resume-provenance.json` |

| Local operation | Block | Envelope bytes | Envelope SHA-256 |
| --- | ---: | ---: | --- |
| Fund private source | 2 | 227,977 | `7e126e1ad406619ebdf178c2d6a6777003c3af51647070d4096582d80ee8cd36` |
| Create and fund vault | 3 | 232,072 | `073df88b32f89da9c7b30faa7aeed667634120ad5feb16b434a2795e9c33739f` |
| Five-of-seven payment | 4 | 232,072 | `fa22ef760a7566927fe2789398e2ec4a44e9fdb0741353b33b05369db47f9771` |

These are local transaction records, not public explorer links. Seven locally
generated test signers enrolled; members 0, 2, 3, 5 and 6 approved the payment;
members 1 and 4 signed rejections. No seven-person human signing exercise is claimed.

## 17 October — checkpoint-backed review/signing

| Evidence | Path |
| --- | --- |
| Validation summary: 12 state-import + 13 workflow + 10 core tests | `lez-approval-bench/lifecycle/client/reports/2026-10-17-state-import/validation-summary.json` |
| Current-note CLI import/review result | `lez-approval-bench/lifecycle/client/reports/2026-10-17-state-import/release-cli-import-review.json` |
| Test output | `lez-approval-bench/lifecycle/client/reports/2026-10-17-state-import/local-checks/` |
| Current-note and spent-note controls | `lez-approval-bench/lifecycle/client/tests/state_import.rs` |
| Source trust boundary | `lez-approval-bench/lifecycle/client/STATE_IMPORT.md` |

The positive state has balance 463 and counter 1. The spent-note control first
checks that the older note still has commitment membership, then requires its
rejection because the update nullifier is present. Real historical receipts are
verified during replay. This milestone generated no new proof, started no new
sequencer and made no public-network submission.

## Reviewer checks on 23 October

`evidence/release-validation.json` records which packaging and archived-binary
checks were actually rerun for this release. It distinguishes them from historical
Rust tests, new proving and hosted CI. No unchecked mode is described as passed.

`scripts/verify_evidence.py` checks byte integrity, source identity, transaction
hash correlation and saved record consistency. A malicious party could replace
both a manifest and its data; obtain the release identity through an independent
trusted channel. Cryptographic verification requires the rebuilt Rust host and
the saved-receipt commands in REPRODUCE.md.
