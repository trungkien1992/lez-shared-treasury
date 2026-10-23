# M = 5 approval-verification result

Date: 7 October 2026. Final trial status: **guest execution passed; real proof not completed or verified**.

The standalone verifier is implemented and builds successfully. All 10 native tests passed, including 15 published BIP-340 vectors with 32-byte messages. The 5-of-7 guest executed and returned the expected journal. A separate real CPU proof attempt was stopped through a terminal interrupt after nearly three hours; no completed receipt was written. The trial is no longer running.

| Measurement | Recorded result |
| --- | ---: |
| Required approvals / members | 5 / 7 |
| Execution-only guest user cycles | 30,197,085 |
| Execution-only segments | 141 |
| Execution-only wall time | 1.588 seconds |
| Guest program binary | 429,084 bytes |
| Segment limit | 2^18 cycles |
| Session limit | 67,108,864 cycles |
| Real prover elapsed at last live snapshot | 10,561 seconds (2 h 56 min 1 s) |
| Real prover cumulative CPU at that snapshot | 41,091 seconds |
| Real prover peak RSS observed by that snapshot | 2,440,308 KiB (2.33 GiB) |
| Completed proof / receipt verification / receipt size | Unavailable |

The real prover was last observed running at 10:16:00 UTC. A terminal interrupt then ended the session with exit code 130; the wrapper, host and prover processes were confirmed absent at 10:16:24 UTC. Interrupting the wrapper prevented its final resource report. The resource figures above come from the last live `ps` and `/proc` snapshot, not post-exit totals or aggregate process memory. They are not a completed proving-time measurement.

The host exposed 9 logical CPUs with a cgroup quota of 8 CPU equivalents and an 8 GiB memory limit. The run configured four Rayon workers on an Intel Xeon Platinum 8370C CPU. No GPU or paid remote prover was used. This was a single exploratory run in a shared execution environment: short host builds and a separate execution diagnostic occurred while proving was active. No median, throughput, SLA, or isolated benchmark is claimed.

## Evidence included

- `unit-tests.log`: 10 native tests passed. Cases include the 15 published 32-byte BIP-340 vectors, distinct quorum, duplicate approvals, wrong member, action binding, stale epoch, bounds and ordering.
- `m5-n7.execution.json`: status `executed_without_proof`, with `is_proving_evidence: false`, the guest image ID, cycles and segment details. The host checked successful halt and expected journal content.
- `approval-guest.bin`: the exact guest program used for execution and the proof attempt. Its SHA-256 and image ID are recorded in `sources.json`.
- `m5-n7.log`: the real-proof start log. It contains no completion or verification event.
- `m5-n7.interruption.json`: operator-stop status, actual observation window, configuration and available live resource observations.
- `build.log`, `verification-build.log` and `diagnostic-build.log`: build records.
- `setup-attempts/`: two failed runtime startups, retained for transparency. Neither reached proof execution; the runtime binary was restored from a checksum-verified official release archive.

There is no `m5-n7.receipt`, verified proof report, or successful saved-receipt verification. `scripts/check_saved_receipt.py` is supplied for a future completed run; its positive and tamper checks were not run because no receipt exists.

## Interpretation and remaining gate

Native tests and guest execution establish limited functional evidence for this fixture. They do not establish a real cryptographic proof, full multisig correctness, LEZ block fit, or completion of RFP P.1/P.2. Stopping the long CPU trial does not establish a prover bug or prove the design infeasible. This artifact does not identify how much longer proving would have taken.

The guest configuration comes from input rather than authoritative LEZ state. A consuming program must establish current account/key ownership, configuration freshness, replay protection, expiry enforcement and exact action execution binding. The journal exposes the threshold, member count and signer indices. Private custody, revocation, coordination, selective disclosure, product interfaces and independent audit are not implemented here.

Next, complete and independently verify a real receipt on an agreed proving environment with a defined time and resource budget. Then integrate with a pinned LEZ private-account execution path and measure complete transactions for M = 1, 3 and 5 against actual network limits. Receipt size must be measured in the required transaction encoding. Keep later milestone pricing and scheduling conditional on these results and the custody/disclosure gates.

The prototype is unaudited. Public deterministic fixture keys must never be used for funds.
