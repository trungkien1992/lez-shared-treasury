# Requirement mapping

This is an evidence/status map for the current prototype, not a claim that every
planned acceptance test is implemented. Final acceptance follows the agreed RFP
and the testnet 0.4 release. F/U/R/P/S labels follow the repository specification.

| Requirement | Current evidence | Remaining acceptance work |
| --- | --- | --- |
| F1 — M-of-N and creation configuration | Creation/enrollment and five-of-seven local payment | General policy/action set and release integration |
| F2 — lifecycle, approve/reject, expiry | Offline requests/responses, approvals/rejections, guest expiry/replay checks | Full product lifecycle, network integration and coordination |
| F3 — arbitrary programs | Native-token transfer only | General call/account binding and target tests |
| F4 — registry | Local decoding for two known instruction variants | Decentralized/versioned target/IDL registry and unknown-target policy |
| F5 — configuration changes | Planned | Quorum-controlled membership and threshold changes |
| F6 — roles | Planned | Independent proposer/approver/executor rules and tests |
| F7 — time lock | Planned | Program-enforced scheduling and execution timing |
| F8 — spending limits | Planned | Atomic allowance/transfer accounting and rollover tests |
| F9 — coordination | Offline file transport only | Encrypted Logos Chat room, identity binding and recovery |
| F10 — privacy and disclosure | Local private-account creation/payment | Public posture and audience disclosure without spending authority |
| U1 — callable module | Rust library/standalone CLI | Logos module API and integration |
| U2 — QML mini-app | Planned | Loadable Basecamp UI, room, designs and builds |
| U3 — headless CLI | Standalone prototype CLI | Logos Core headless framework and all essential operations |
| U4 — SPEL IDL | Planned | Generated program IDL and drift checks |
| U5 — privacy documentation | Prototype trust/scope notes | Complete per-action/posture disclosure tables |
| U6 — exact-byte signing display | Local canonical decoding and review-digest checks | QML interface, full registry coverage and UI checks |
| U7 — errors | Prototype error paths and CLI controls | Full operation/error mapping and product behavior |
| R1 — distinct current approvals, exact action, no replay | Shared core and supported lifecycle tests | Full action set, configuration transitions and role matrix |
| R2 — configuration cutoff | Proposed epoch policy in proposal | Program implementation and removal/delay/race tests |
| P1 — complete operation costs | Dated local proving records | Every release operation measured against 0.4 limits |
| P2 — threshold curve, M >= 5 | Local five-of-seven feasibility evidence | Full curve, largest viable M and reproducible release hardware report |
| S1 — LEZ private accounts | Local private PDA evidence | Release integration on the selected runtime |
| S2 — Logos Chat | Planned | Actual module integration |
| S3 — testnet 0.4 compatibility | Not demonstrated | Pinned 0.4 builds and acceptance evidence |
| S4 — testnet 0.4 deployment | Not demonstrated | Public deployment and transaction records |
| S5 — sequencer integration in CI | Local replay driver; historical node evidence | Hosted sequencer CI job with actual result |
| S6 — green default branch | Workflow prepared | Successful hosted run on published commit |
| S7 — tests for every F/U/R item | Supported subset has automated checks | Complete requirement-to-test mapping at release |
| S8 — end-to-end README | Local reproduction instructions | Full CLI/UI/network/room operator journey |
| S9 — module API documentation | Prototype Rust API notes | Full module developer integration docs |
| S10 — CLI docs | Prototype CLI commands | Essential release journeys on headless framework |
| S11 — GUI designs | Planned | Editable designs or equivalent |
| S12 — catalog | Planned | Team catalog and installable modules |
| S13 — independent audit | Prototype scoping/estimate | Agreed full-release scope, frozen commit, review, remediation and public reports |
| S14 — decision log | Client decisions and lifecycle design retained | Complete current programme decision log |
| Privacy 1 — chain attribution | Private local fixture scope documented | Release transaction inspection for all action classes |
| Privacy 2 — transport attribution | No Chat adapter | Actual transport tests and identity-boundary analysis |
| Privacy 3 — correlations | Prototype limits documented | Full chain/coordination correlation inventory |

Weighted approvals, proof-of-holding inequalities and mixed postures are not
implemented by this prototype. Any optional extension requires an explicit scope
decision; it cannot be substituted for the hard requirements above.
