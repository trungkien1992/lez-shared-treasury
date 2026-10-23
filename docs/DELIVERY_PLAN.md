# Proposed delivery plan — testnet 0.4

Planning update dated 23 October 2026 for discussion with Logos. This is not an
executed grant agreement or an audit booking. The published proposal remains
[issue #211](https://github.com/logos-co/rfp/issues/211).

## Acceptance target and historical evidence

The current repository specification makes testnet 0.4 the final compatibility,
deployment and benchmark target:
[RFP-005](https://github.com/logos-co/rfp/blob/master/RFPs/RFP-005-multisig-and-coordination.md),
reviewed at file SHA `646c8cd4d7aea938b7029b252135bed29b787b92`.

The implementation in this evidence release remains on its original pinned LEZ
revision. Preserve it as the historical feasibility baseline. A future port must
have its own pinned dependencies, guest identity, real proofs, transaction records
and acceptance report; old receipt verification does not validate a new runtime.

M1 should identify the usable 0.4 source/release and integration interfaces. If
the required release is unavailable, report the dependency and agree a delivery
gate with Logos. Do not silently label a 0.3 or local result as 0.4 acceptance.

## Milestones and allowance

| Milestone | Proposed payout | Planning duration | Acceptance focus |
| --- | ---: | --- | --- |
| M1 — feasibility and architecture | $8,000 | Weeks 1–3 | Public reproducible benchmark and custody/state/identity feasibility evidence; version and staffing plan; refine later estimates |
| M2 — multisig program and policies | $24,000 | Weeks 4–9 | General lifecycle, configuration, roles, time locks, spending limits, arbitrary actions and tests |
| M3 — coordination and disclosure | $18,000 | Weeks 10–14 | Account/chat binding, encrypted rooms, reconciliation, key migration and audience verification |
| M4 — module, CLI and mini-app | $20,000 | Weeks 15–19 | Logos interfaces, QML, decoder/registry, installable builds and draft documentation |
| M5 — integrated candidate | $7,000 | Weeks 20–21 | Testnet 0.4 deployment and real benchmarks, complete requirement mapping, green sequencer CI and frozen candidate |
| M6 — independent audit | $35,000 allowance | Weeks 22–25 | Agreed provider and full-release scope, independent assessment and findings triage |
| M7 — remediation and delivery | $8,000 | Weeks 26–28 | Fixes, retest, public final reports, release, catalog, handover and 90-day defect support |
| Total | $120,000 | Approximately 28 weeks | Later scope, staffing and estimates confirmed after M1 |

These retain the existing proposal amounts. The allowance is not an obligation
to spend the full amount or a representation of a full-release vendor quote.
Payment follows accepted milestones under an executed agreement. Existing
pre-application work is disclosed as baseline evidence; M1 must identify the
remaining work being accepted and must not represent old work as new activity.

## Remaining M1 questions

1. Benchmark approval verification across M and the complete private transaction
   under the selected runtime, padding and client hardware; establish the viable
   threshold against actual network limits.
2. Select and test an authenticated checkpoint/live-state path, including rollback,
   restart and stale-note behavior. Distinguish consensus/finality authentication
   from the already implemented local replay verifier.
3. Demonstrate custody enforcement with the supported private account/key model,
   and establish member ownership, removal, key migration and disclosure boundaries.
4. Exercise the actual Logos Chat identity, provisioning and reconciliation APIs.
5. Confirm contractor capacity, release dependencies and acceptance criteria for
   M2–M7. A well-supported negative feasibility result is reported rather than
   described as full RFP completion; its payment treatment needs agreement.

## Audit schedule and scope

Hacken's preliminary prototype estimate is $5,000 for three business days of
secure code review and $12,500 for six business days of cryptography review.
The workstreams would run in parallel. QA and a preliminary report follow, then
a 20-business-day remediation period that includes retesting, followed by final
report delivery and publication. These are scoping statements, not reserved
calendar dates or a guaranteed final-report turnaround.

For the proposed 28-week plan to work, findings should be triaged and fixes
started as they arrive during M6. A candidate planning window is assessment/QA
in weeks 22–23, rolling remediation/retest through week 27, and final reporting
and handover in week 28. Waiting until week 26 to begin the full 20-business-day
window would exceed the three-week M7 block. Agree the overlap and actual QA,
retest and publication dates with both Logos and the auditor; extend or rephase
the schedule explicitly if needed.

The release scope must cover the multisig, approval verification, configuration,
time locks and spending policies required by S13, plus client, host, state import,
decoder and coordination boundaries that can affect authorization. Distinguish
covered source from exclusions and changes introduced after the audit freeze.
The current prototype quote does not price every future feature in this list.

The public proposal names Trail of Bits as a provisional first choice. Hacken is
an alternative under consideration; Logos must agree the provider, scope and
budget before an appointment is represented as committed. Hacken has allowed
sharing a scope/pricing summary and confirmed public final-report hosting. The
full commercial proposal and private document URL are not part of this release.
At contracting, ensure the agreed report publication/repository-linking terms
meet Logos' requirements. No mainnet or production recommendation precedes the
independently reviewed release and completed retesting.
