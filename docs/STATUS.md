# Implementation status — reviewer release, 23 October 2026

The latest implementation milestone is 17 October. Packaging and documentation
on 23 October do not turn the earlier observations into a new proving run.

| Component | Current state | Evidence or next gate |
| --- | --- | --- |
| Canonical approval bytes and distinct eligible signers | Implemented for the supported creation/native-transfer subset | Shared core tests and client workflow tests |
| Creation, unanimous enrollment and optional funding | Real local proof and sequencer evidence | 13 October creation receipt and block-3 envelope |
| Five-of-seven native-token transfer | Real local proof and sequencer evidence | 13 October spend receipt and block-4 envelope |
| Offline approve/reject collection | Implemented and locally exercised | Seven local test signers; six CLI negative controls |
| Vault-state import and verified review/signing | Implemented relative to a separate trusted checkpoint | 12 state-import tests and recorded CLI import/review |
| Independently authenticated live checkpoint | Not implemented | Select and validate a supported source; document finality and rollback policy |
| Automatic wallet state synchronization | Not implemented | Live adapter and restart/reorg handling |
| Secure account-key distribution, ownership and rotation | Not implemented | Funded feasibility and design gate |
| Configuration changes, roles, time locks and spending limits | Planned | Program implementation and requirement tests |
| Arbitrary actions and full decoder registry | Planned | Target/IDL identity checks and integration tests |
| Encrypted Logos Chat room and identity binding | Planned | Real multi-client provisioning, membership and recovery tests |
| Selective disclosure and public posture | Planned | Audience verification without spending authority |
| Logos headless module, QML mini-app, SPEL IDL and catalog | Planned | Platform-integrated clients and installable releases |
| Public testnet deployment | Not demonstrated | Final delivery and benchmarks target testnet 0.4 |
| Hosted CI | Fresh source build, 35 Rust tests and seven-signer CLI demo passed on 23 October | See [GitHub Actions](https://github.com/trungkien1992/lez-shared-treasury/actions) for the complete workflow result; the first run exposed a measurement-wrapper portability issue, corrected in the Python harness |
| Full threshold performance curve | Not completed | Real proving across M, wallet padding and measured network limits |
| Independent audit | Preliminary scoping only | Accepted provider, release inventory, frozen commit, contract and schedule |

The archived `completed: true` flag in the original status file describes its
named local milestone, not completion of RFP-005. The original is retained in
`docs/provenance/status-2026-10-17.json` for provenance.
