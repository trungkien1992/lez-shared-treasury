# Real lifecycle and local sequencer results — 12 October 2026

The unchanged lifecycle guest generated real creation/funding and 5-of-7 spend receipts. Both saved succinct receipts verified in a separate process. An unmodified pinned LEZ standalone sequencer included native private funding, vault creation and the subsequent spend, and returned the exact submitted transaction bytes. Its accepted membership witness was ingested by the host and matched the proved spend input.

## Actual node inclusion

| Operation | Block | LEZ envelope bytes | Transaction hash |
| --- | ---: | ---: | --- |
| fund-private-source | 2 | 227,977 | `54905db89319e187295183ce93550339d8831d830aa5684cdfbaf10351b82684` |
| create-and-fund-vault | 3 | 232,072 | `06f19c257ad4d984faaf34fd885014f22a7023c448b59d0bffec6745ca9ce81a` |
| five-of-seven-spend | 4 | 232,072 | `36c982b7681e572be93a3c8ce6333a9c87ff824e3f0fbd27f73e562971d15671` |

The node ran on loopback with gossip disabled and development mode off. Upstream revision: `411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4`. Sequencer binary SHA-256: `91aa8741b222c4878f767ad950481e3513ac6ed8a74351f4e900cb7a6989790e`. Its standalone mode uses upstream's mock Bedrock; this is not public-devnet or L1-finality evidence. Node submission and observation took 8.23 seconds, excluding proof generation.

## Account state

| Step | Public test source | Private funder | Treasury | Recipient | Treasury proposal counter |
| --- | ---: | ---: | ---: | ---: | ---: |
| Test genesis | 1,000,000 | Absent | Absent | Absent | Absent |
| Native private funding | 999,000 | 1,000 | Absent | Absent | Absent |
| Create/fund treasury | 999,000 | 500 | 500 | Absent | 0 |
| Five-member-approved spend | 999,000 | 500 | 463 | 37 | 1 |

Private values come from independently decrypted proof outputs whose exact transactions were included. Accepted commitments, the creation membership path and final private root matched the proof workflow. The tracked test allocation sums to 1,000,000 throughout; unrelated upstream genesis accounts are outside this table.

## Proof measurements and reuse

Guest SHA-256: `fff9878b2215978e6be4bd62ac1f89a4c3b1b351bf4f0d61b308eaa0c30e321c`. It is the exact archived creation/enrollment guest. Guest source and the original validation source remained unchanged. No stub or fake receipt was used.

| Operation | Stage | Recorded seconds | Provenance |
| --- | --- | ---: | --- |
| creation | vault-plan | 36.01 | verified checkpoint reused |
| creation | vault-apply | 709.06 | verified checkpoint reused |
| creation | ppe-composite | 725.05 | fresh real proving |
| spend | vault-plan | 46.44 | verified checkpoint reused |
| spend | vault-apply | 578.34 | verified checkpoint reused |
| spend | ppe-composite | 707.76 | fresh real proving |
| creation | succinct compression | 947.34 | fresh compression and verification |
| spend | succinct compression | 790.19 | fresh compression and verification |

The node-matching invocation took 52.86 minutes, reusing the already-verified plan/apply receipts and generating both PPE receipts and compression afresh. This is not a fresh complete end-to-end benchmark. Earlier approval-stage timings are retained individually. Native private funding was separately proved and compressed; its receipt was reused after input-hash verification for the node-matching profile. Runtime/resource JSON records hardware, process memory definition, timeouts and concurrency. Several earlier stages shared the machine with builds or another prover; these are feasibility observations, not production throughput estimates.

Both treasury envelopes are 232,072 bytes. Actual local-node inclusion confirms admission under the configured 1 MiB block limit. Larger M, intended wallet padding, and public-network limits were not measured here.

## Validation

- All 47 existing negative cases passed again in both the isolated and node-matching state profiles.
- Each proved transaction passes the valid transition and rejects exclusive expiry, modified proof-bound messages and replay while preserving rejected-transition state.
- A spend witness is unavailable before creation; the proved spend is rejected against pre-creation state.
- Separate saved-proof verification checks receipt identity, journal/transaction binding, both encodings, accepted state snapshots and spend-witness bytes.
- The node's funding root, accepted creation membership path and final spend root match the verified proof workflow.
- Altered node membership and altered saved transaction controls fail for their expected reasons; original evidence remains unchanged.
- The six read-only preflight checks pass. The genesis-export helper reproduced the exact snapshot bytes.

## Integration correction

The minimal isolated fixture did not contain the upstream node's preinstalled private accounts and immutable program mirrors. Its proof is valid for that fixture, but its update witnesses are unsuitable for the full node tree. The final workflow loads `testnet_initial_state::initial_state(false)` from the exact built upstream library. Its exported root matches the observed fresh node. Creation/spend PPE proofs were regenerated for that tree; approval-stage receipts were reused only after their input hashes matched.

The node itself was not patched or populated with simulated treasury commitments. The normal native funding transaction established the private source account before vault creation. The model carries forward creation through proof-verified state transition, and the node returns an independently matching membership witness.

## Scope and reproducibility

Start with `REAL_WORKFLOW.md`, `scripts/show_results.py`, and the dated result JSON. Reports use `--state-profile standalone` and the bundled genesis snapshot. A rebuilt guest is optional; the host's prebuilt mode checks the exact archived guest hash. Sources, lockfiles, receipts, encoded transactions, checkpoint metadata and logs are retained.

This completes a controlled local fixture journey. All keys are public deterministic test material, the application domain is synthetic, and each treasury transaction uses two real notes, zero dummy notes and 1024-byte ciphertext padding. General wallet synchronization/key sharing, LEZ-identity enrollment, membership/key rotation, selective disclosure, encrypted coordination, policy features, product CLI/QML, public devnet/testnet compatibility and independent audit remain pending. No grant milestone acceptance, production readiness or mainnet safety is inferred.
