# Staffing and responsibility plan

Peter Nguyen is the principal builder and sole currently committed engineer.
No additional person is appointed, and no availability or delivery capacity is
claimed for a prospective contractor. The later milestone estimates are
conditional on M1 evidence and agreed capacity.

| Workstream | Current owner | Assistance to confirm | Evidence needed before commitment |
| --- | --- | --- | --- |
| Treasury invariants, program and release | Peter | Scoped Rust/LEZ engineer or reviewer | Relevant source work, review exercise, weekly capacity and written scope |
| Wallet and live state | Peter | Rust/zkVM specialist | Pinned LEZ integration experience, complete-state/finality design and reproducible test work |
| QML and Basecamp integration | Peter until appointed | Qt/QML/Logos client engineer | Loadable sample, C ABI/module experience, exact-byte signing UI exercise and confirmed availability |
| Security assessment | Unappointed independent provider | Provider accepted by Logos | Written scope, frozen inventory, budget, report terms and slot |

## Bounded specialist tasks

The Rust/LEZ task is to review and prototype the authenticated live-state adapter,
rollback/restart handling, private-account ownership and key distribution on the
selected 0.4 interface. Deliver a minimal reproducible integration, tests and an
explicit trust/assumption note. It does not include an unbounded production
wallet build before funding or replace the independent release audit.

The QML task is to build a small loadable Basecamp module around the shared Rust
client API, displaying the exact reviewed instruction, state trust/freshness,
approval progress and proof status. First acceptance is a sample integration
and documented build, followed by a scoped mini-app plan.

During M1, confirm a rate or fixed scope, weekly hours, dates, deliverables, review
responsibility, licensing and handover. Do not publish a person's name as a team
member before they agree. If capacity cannot be secured within the existing
reserve, adjust later scope or timing with Logos before committing to it.

The staffing gap remains open. This document makes the allocation and selection
criteria concrete; it is not evidence that a specialist has accepted an offer.
