# Member workflow completed — 13 October 2026

The offline Rust member client completed its local proof and settlement milestone. Seven fresh local test signers reviewed canonical instruction bytes, signed unanimous enrollment, and supplied five distinct nonconsecutive spending approvals plus two signed rejections. The collector verified the responses without reading a member key file and exported the exact instructions consumed by the unchanged LEZ guest. This was an automated local demonstration, not signing by seven human team members.

The new creation and payment receipts passed a separate saved-file cryptographic check. A freshly started, unchanged pinned standalone sequencer included the native funding, member-driven vault creation and 5-of-7 payment. The included transaction bytes matched the saved envelopes exactly. The host ingested the actual node's creation membership witness and verified that its bytes matched the input used for the payment proof. The node shut down with exit code 0.

The private funder ended at 500 base units, the vault at 463 and the recipient at 37. Creation deposited 500 into the vault; payment sent 37 and advanced the proposal counter from 0 to 1. All four output notes were independently decrypted and checked. The public bootstrap account's balance changed from 1,000,000 to 999,000.

| Verification | Result |
| --- | --- |
| Member-client tests | 13 passed |
| Shared approval-core tests | 10 passed |
| CLI negative controls | 6 passed |
| Updated default host/guest rejection suite | 47 passed |
| Saved creation and payment receipts | 2 independently verified in a fresh process |
| Actual local sequencer inclusions | 3; exact saved bytes |
| Node creation membership witness | Ingested; exact proved payment input matched |
| Shared core, guest and measured prover | Byte-equal to the October 12 baseline |
| Fresh member signing secrets | Excluded from project and reports |

Approving spend member indices were `[0, 2, 3, 5, 6]`; signed rejecting indices were `[1, 4]`. Duplicate delivery counted each signer once. Rejection is a signed opinion rather than a veto; the configured five-of-seven approval quorum still passed. Negative controls rejected insufficient approvals, an altered amount, relabeling a rejection, stale context, an incorrect review digest and an unsafe key-file permission mode.

## Proof provenance and timings

Only two verified creation approval checkpoints were reused from the earlier trial of this same new member workflow. Their exact input hashes and real receipts were rechecked before reuse. Creation PPE, payment stages and both succinct compressions completed in the resumed eight-worker trial. No October 12 application receipt was reused to prove the new member configuration. The historical native-funding receipt was reused because the genesis, private funder and bootstrap amount were unchanged.

| Operation | Stage | Recorded proving seconds | Provenance |
| --- | --- | ---: | --- |
| creation | vault-plan | 64.851 | Verified checkpoint from this same workflow |
| creation | vault-apply | 1356.349 | Verified checkpoint from this same workflow |
| creation | ppe-composite | 871.660 | Generated in resumed trial |
| spend | vault-plan | 32.684 | Generated in resumed trial |
| spend | vault-apply | 520.782 | Generated in resumed trial |
| spend | ppe-composite | 823.375 | Generated in resumed trial |

| Operation | Succinct compression seconds | LEE envelope bytes |
| --- | ---: | ---: |
| creation | 1103.581 | 232,072 |
| spend | 933.738 | 232,072 |

The resumed trial took 4287.285 seconds and recorded a maximum child-process RSS of 3,660,012 KiB. This wall time excludes the work represented by reused checkpoints and is not an uninterrupted fresh end-to-end benchmark. RSS is the Linux maximum per child process, not simultaneous aggregate memory. The interrupted first trial has no complete resource report. Read `resume-provenance.json` and the stage metadata for exact input and receipt hashes. No GPU, fake receipt or paid remote proving service was used.

## Actual node inclusions

| Operation | Transaction hash | Envelope bytes |
| --- | --- | ---: |
| fund-private-source | `54905db89319e187295183ce93550339d8831d830aa5684cdfbaf10351b82684` | 227,977 |
| create-and-fund-vault | `745c704e799195b2fad42c769cfc5bc53725059daf7f4f24bd7cbf1e59e080c5` | 232,072 |
| five-of-seven-spend | `30f0bd6e46c4be4b859a3a2fa1323394d22306ef06b8d946d20fbf8e6b1be7b0` | 232,072 |

The node used loopback RPC, disabled gossip and mock Bedrock. Its source pin was `411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4`. The node binary SHA-256 was `36ba6487a16957161f70ec91d0a5b6ed805b1f5d9b317de9e7e8449b7646027b`. Exact channel, block IDs, commitment roots, RPC observations and decrypted-note evidence are retained in the dated reports.

## Evidence and reproduction

- `reports/2026-10-13-member-workflow/`: public keys, context, requests, signed responses, controls, exact exports, original transcript and completed `verification.json`.
- `../reports/2026-10-13-client-guest-execution/`: execution-only check of those exports; clearly separate from proving and submission.
- `../reports/2026-10-13-client-real-lifecycle/`: real receipts, encodings, stages, resumed-run provenance, independent receipt check and actual node witness check.
- `../reports/2026-10-13-client-settlement/`: fresh node configuration, log, RPC observations and successful settlement report.
- `reports/2026-10-13-member-workflow/local-checks/`: test logs and unchanged-source/secret-exclusion checks.

Guest SHA-256: `fff9878b2215978e6be4bd62ac1f89a4c3b1b351bf4f0d61b308eaa0c30e321c`. Creation instruction SHA-256: `eade7001a2c6bdc23dfbc33c9accf4971f275b5252f846ef8d64e8199c166239`. Payment instruction SHA-256: `1cde421a637feaf2630327f680dbeba57d22dc51715bc920236c4f049835a18a`. The archived guest image and original core source were not modified. `README.md` provides CLI, saved-evidence checks and fresh node reproduction commands. Archive integrity checking is separate from cryptographic receipt verification.

The original demo transcript and export JSON describe their generation time, before external proof and submission. The later `verification.json` and settlement report record completion. The latest binary reports external execution tracking rather than claiming local export itself is settlement. Hosted CI has not been run; its equivalent quick checks passed locally.

## Remaining scope

This completes a bounded local member-client milestone, not the full RFP or a production treasury. LEZ viewing/nullifier/account material remains public test fixtures. Context synchronization and account ownership are not authenticated by the client; the proving bridge retains fixed test accounts and amounts. Signing keys use a local development file format, not an encrypted production keystore.

Logos Chat/E2EE and room binding, authenticated wallet/state synchronization, secure viewing/nullifier key sharing, Basecamp QML/headless-module packaging, arbitrary target actions, configuration changes, roles, spending policies, disclosure modes, public devnet validation, larger threshold measurements and independent audit remain open. Public-network compatibility cannot be inferred from this standalone result.
