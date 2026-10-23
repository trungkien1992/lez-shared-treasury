# LEZ Shared Treasury

A local Rust prototype for a private treasury on the Logos Execution Zone.
It demonstrates unanimous treasury enrollment, a five-of-seven native-token
payment, member approval/rejection collection, and checkpoint-backed state
verification before signing.

This is the evidence release for [Logos RFP-005 proposal #211](https://github.com/logos-co/rfp/issues/211).
The implementation snapshot is dated **17 October 2026**. This reviewer package
was prepared on **23 October 2026**. It is an unaudited development prototype
using local test accounts, with substantial RFP work still planned.

## Review in five minutes

1. Run `python3 scripts/verify_evidence.py` to check package integrity and the
   links between the saved client, proof and local-node records. Python 3.11+
   is sufficient; this check does not cryptographically verify RISC Zero proofs.
2. Read [the evidence index](docs/EVIDENCE.md) for the exact completed runs.
3. Read [the implementation status](docs/STATUS.md) and
   [the trust model](docs/TRUST_MODEL.md) for the limits of each result.
4. Follow [reproduction instructions](docs/REPRODUCE.md) to build from source,
   verify saved real receipts, run fresh client controls or replay local settlement.
5. Review [the delivery plan](docs/DELIVERY_PLAN.md) and
   [requirement mapping](docs/REQUIREMENTS.md) for the proposed testnet 0.4 release.

## What the evidence establishes

| Area | Evidence | Limit |
| --- | --- | --- |
| Private creation and payment | Saved real RISC Zero receipts; three exact encoded transactions accepted by a standalone LEZ sequencer on 13 October | Local mock-Bedrock environment; no public-testnet acceptance claimed |
| Member workflow | Seven local random test signers enroll; five approve a 37-unit payment and two sign rejections; collector exports guest-compatible bytes | Offline files, development key storage, no human team or Chat integration |
| State before signing | Receipt-verified replay, complete-state hashes, spent-note rejection, checkpoint-bound review/signing | The operator supplies an independently trusted checkpoint and local nullifier key |
| Validation | 35 Rust tests passed in a fresh GitHub source build on 23 October; seven-signer demo and six CLI negative controls also passed | See [GitHub Actions](https://github.com/trungkien1992/lez-shared-treasury/actions) for the complete workflow result; fresh proof generation and node inclusion remain separate |

The final local balances were 500 for the private funding account, 463 for the
vault and 37 for the recipient, with proposal counter 1. They are local test units.
The creation/spend workflow reused two verified creation-stage checkpoints and a
historical funding receipt. Its timings are not an uninterrupted fresh benchmark.

The latest client gate rejects an older spent note even when its commitment
membership proof still exists. Its guarantee is relative to the supplied trusted
checkpoint; it does not authenticate network consensus, finality or the checkpoint
source, and it does not reserve the note against subsequent spending.

## Source map

| Path | Purpose |
| --- | --- |
| `lez-approval-bench/core/` | Shared canonical approval verification and BIP-340 vector tests |
| `lez-approval-bench/lifecycle/core/` | Treasury initialization and native-token transfer rules |
| `lez-approval-bench/lifecycle/methods/guest/` | RISC Zero guest executing the lifecycle program |
| `lez-approval-bench/lifecycle/host/` | Execution, real proving, saved-receipt checks and local fixtures |
| `lez-approval-bench/lifecycle/client/` | Offline Rust member library/CLI, checkpoint importer and tests |
| `lez-approval-bench/integration/` | Earlier integration harness and shared measurement helpers |
| `scripts/` | Reviewer checks, build/test entry point and saved-proof verification |
| `docs/provenance/` | Original archive identity, unchanged source hashes and omitted-file list |

All 31 Rust files and Cargo manifests/lockfiles retain their original snapshot
bytes. Large intermediate proving receipts, compiled Linux host/client binaries
and demonstration media are omitted. Final succinct receipts, encoded transactions,
state fixtures, guest images, test reports and dependency notices are included.

## Versions and next delivery

The historical evidence pins LEZ revision
`411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4`, RISC Zero 3.0.5 and the recorded
guest SHA-256 `fff9878b2215978e6be4bd62ac1f89a4c3b1b351bf4f0d61b308eaa0c30e321c`.
The host Rust toolchain is 1.98.1. Do not substitute a newer LEZ revision and
describe that as a reproduction of these results.

The proposed final delivery targets **LEZ testnet 0.4**, including its deployment
and performance limits. That port and public-network validation are future work;
the archived local results establish neither compatibility nor completion.

## Team and audit planning

Peter Nguyen is the principal builder and sole currently committed engineer.
Rust/LEZ and Qt/QML assistance is planned subject to funding; no contractor is
represented as appointed. [Staffing responsibilities](docs/STAFFING.md) describe
the gaps and the checks needed before later milestones are confirmed.

The proposal requests $120,000, including an $8,000 initial feasibility milestone
and a provisional $35,000 audit allowance. Hacken has provided a preliminary
$17,500 estimate for the listed prototype scope, including the Rust client.
Full-release scope, provider acceptance, final price and scheduling require
agreement and a frozen candidate. No audit engagement is represented as booked.

## License and contact

Project code is dual licensed under MIT OR Apache-2.0; see [LICENSE-MIT](LICENSE-MIT)
and [LICENSE-APACHE](LICENSE-APACHE). Dependencies and upstream code retain their
own notices. See [THIRD-PARTY-NOTICES](lez-approval-bench/THIRD-PARTY-NOTICES.md).

Private security reports: see [SECURITY.md](SECURITY.md). Development and grant
questions can be raised in [proposal #211](https://github.com/logos-co/rfp/issues/211).
