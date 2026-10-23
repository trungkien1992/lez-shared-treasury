> **13 October member-client milestone completed:** independent byte-derived review, signed approvals/rejections, real creation/payment receipts and actual local sequencer inclusion. Read [client/CLIENT_RESULTS_2026-10-13.md](client/CLIENT_RESULTS_2026-10-13.md) and [client/README.md](client/README.md). Public devnet and full product delivery remain pending.

# Devnet validation handoff

The October 12 baseline included three exact transactions on a fresh, unchanged pinned standalone sequencer: native funding, vault creation and payment. That node uses mock Bedrock and loopback RPC. The October 13 member-client results record the separate run using fresh local approval signers. No public-network submission has been performed. A reachable public LEZ sequencer endpoint, its confirmed channel and deployed runtime have not been established. A Bedrock/L1 endpoint alone is not a LEZ sequencer RPC endpoint.

## Prepared read-only check

`scripts/devnet_preflight.py` verifies the upstream Git pin, clean source and PPE ELF checksum, then compares the archived v1 envelope with the pinned repository configuration. It can also call only these documented methods: `checkHealth`, `getChannelId`, `getLastBlockId`, `getProgramIds`, and `getProofsAndRoot` with an empty commitment list. It contains no transaction-submission path. Local HTTP tests cover request framing, an incorrect channel and rejection of a write method.

After independently confirming an endpoint and channel, run:

```sh
python3 scripts/devnet_preflight.py \
  --rpc-url "$LEZ_RPC_URL" \
  --expected-channel "$LEZ_CHANNEL_ID_HEX" \
  --output reports/devnet-preflight-confirmed.json
```

The endpoint must be HTTPS unless it is loopback. The script does not accept credentials in a URL. The expected channel must come from confirmed operator/deployment settings, not from blindly accepting the endpoint's own response. Success of these read calls alone does not verify node software compatibility, proof-circuit identity, live size limits, state synchronization or transaction acceptance. The report retains these open checks and exits with status 2 while this prototype remains incomplete.

## Required next integration work

1. Confirm the LEZ sequencer endpoint/channel, software revision, accepted PPE image, proof format and maximum transaction size. For a local sequencer, use the pinned upstream startup/configuration instructions and fresh local state. Do not silently replace the pinned runtime with a newer network revision.
2. Generate fresh test-only member, viewing and nullifier material using the eventual wallet/key-sharing implementation. Agree the descriptor, network domain, program image and expiry through the member review interface, then collect every enrollment. Public fixture keys and synthetic network domains are unsuitable for a live vault.
3. Fund an independent test account and synchronize its actual private state and commitment membership proof. The local fixture's source balance and root cannot be used on a live chain. Derive the vault from the agreed descriptor before the first deposit.
4. Adapt the bounded local proving bridge to authenticated wallet state on the confirmed deployment. The host already generates and independently verifies real creation/funding and payment proofs, and checks the actual local node membership witness against the proved spend inputs. Its current fixed test-account material, initial funding and genesis witnesses cannot be reused on a public network.
5. Use the upstream typed RPC client to submit newly constructed transactions on the confirmed test deployment, then verify inclusion, decrypted post-state, expiry, repeated submission and rejected operations. Record node revision, channel, block IDs, transaction hashes, costs and exact guest/receipt hashes.

The binary `.lee-envelope` in the earlier integration report is a Borsh size/encoding artifact, not a JSON-RPC request body. `sendTransaction` accepts upstream `common::LeeTransaction` through its typed RPC interface. Its payload depends on live witnesses; never submit the archived synthetic-genesis transaction.

## Sources checked

All code references below are pinned to `411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4` in the official [LEZ repository](https://github.com/logos-blockchain/logos-execution-zone/tree/411adc8fdb3f4c3af64354c4fc26d3e4a6a3d3f4).

- `README.md`: local services and Docker startup.
- `lez/sequencer/service/rpc/src/lib.rs`: method names and typed request/response contracts.
- `lez/sequencer/actors/rpc_server/src/actor/service.rs`: transaction encoding, stateless checks and admission.
- `lez/sequencer/core/src/config.rs`: 2048-byte block overhead used by the admission size calculation.
- `lez/configs/docker-all-in-one/devnet/sequencer_config.json`: repository 1 MiB block configuration and 20-transaction count limit, not verified live limits.
- `integration_tests/tests/private_pda.rs`: actual funding, synchronization and PDA spend integration patterns.

Read `reports/devnet-preflight-offline.json` for the observed environment and remaining blockers. The recorded October 12 creation and payment envelopes each measure 232,072 bytes and fit the pinned repository's 1,046,528-byte single-transaction comparison cap. The dated October 13 proof report records its own encodings. These local measurements do not establish a public deployment's runtime, proof compatibility or current admission limits.
