> **13 October member-client milestone completed:** independent byte-derived review, signed approvals/rejections, real creation/payment receipts and actual local sequencer inclusion. Read [client/CLIENT_RESULTS_2026-10-13.md](client/CLIENT_RESULTS_2026-10-13.md) and [client/README.md](client/README.md). Public devnet and full product delivery remain pending.

# Creation and enrollment contract

Decision recorded 7 October 2026. This extends the bounded feasibility prototype; it does not change the submitted proposal or claim completion of F.1/F.9.

## Canonical descriptor and identity

`Descriptor` is Borsh encoded in this order: version, 32-byte network domain, 32-byte per-vault salt, spending threshold, ordered member public keys, nullifier public key, viewing public key, and account identifier. Initial configuration epoch is fixed at 1 and the first proposal counter is 0. Supported semantic bounds are 1–64 distinct valid keys and a threshold from 1 through the member count.

The custody seed is SHA-256 of `LEZ-RFP005/vault-descriptor/v1\0` followed by the exact descriptor bytes. The actual vault address comes from upstream `AccountId::for_private_pda(program, seed, npk, vpk, identifier)`. There is no caller-selected independent custody seed. Changing the member set, order, threshold, network, salt, public key material or identifier changes the descriptor commitment and address. The program checks the derived address against the configuration handle before initialization.

This binds the configuration before funding. A deposit sent to the agreed descriptor-derived address cannot be redirected to a different initial member set by choosing another initialization payload. Initialization also rejects every nonempty configuration shard, including malformed existing data; there is no reset/delete route.

## Member consent

Every listed initial member must enroll. The eventual spending threshold does not lower this enrollment requirement. This decision avoids listing a member without their signing-key consent; its availability cost is that one absent initial member prevents creation. A different creation policy would need an explicit future protocol change.

The enrollment digest uses the existing approval verifier with an N-of-N verification configuration. The action binds:

| Signed field | Value |
| --- | --- |
| Network | Descriptor network domain |
| Instance | Derived private vault address |
| Program and target | This exact program address |
| Epoch | 1 |
| Proposal identifier | Descriptor digest |
| Expiry | Exclusive upper bound in Unix milliseconds |
| Instruction | `LEZ-RFP005/member-enrollment/v1\0` plus the complete descriptor |

Each response contains a member index and a 64-byte BIP-340 signature. The verifier rejects missing, repeated, extra, invalid or nonmember responses. The signature binds the requested spending threshold through the complete descriptor, even though enrollment verification itself requires N-of-N. Spending uses the distinct native-transfer action domain and target; enrollment signatures cannot be repurposed as transfer approvals.

The selected program uses LEZ's shadow address derived from its RISC Zero image ID. Changing the compiled guest changes the program identity, vault address and signed action. The previous integration receipt does not prove this new guest.

## Atomic creation and optional funding

The creation Plan emits one configuration effect. Apply checks the empty pre-state, descriptor/address binding and all enrollment signatures, then writes the same `VaultState` structure consumed by the archived transfer verifier. LEZ dispatches planned chained calls only after the approval effect succeeds.

Creation without funding selects only the configuration shard. Creation with funding additionally selects the vault native balance and the funding account's native balance. The funding source must be distinct, the amount positive, and the funding handle authorized. A normal native-token call performs the debit and credit; the creation program grants no PDA spending authority over the funding account. The private funding fixture supplies its real authorization witness. Missing authorization and insufficient funds both abort the flow.

The optional inbound source and amount are intentionally outside the member enrollment signature. They do not change the agreed configuration or vault address; the funding account independently authorizes the exact executed transaction. Client displays must distinguish member enrollment approval from funding authorization. Any future operation that spends existing vault funds must use the separate M-of-N path.

## State, time and privacy boundaries

The new dispatcher preserves the original tagged Apply input in its journal when forwarding a transfer to the archived core. The core reads committed pre-state, verifies the current configuration/epoch/sequence/seed, increments the proposal counter and authorizes the native custody call.

Plan places the enrollment/transfer expiry into LEZ's timestamp validity window. Guest execution checks this propagation, but it has no trusted wall clock. The October 12 real lifecycle receipts passed local state-transition checks for expiry and replay, and the exact transactions were included on the pinned standalone sequencer. The dated October 13 member-client results separately record the new signer workflow. Public-network validation remains pending.

The network domain is an application/client assertion. The pinned Plan/Apply API does not supply a chain identifier. Merely signing a domain does not make a transaction unconditionally unreplayable across identical or cloned chains. Clients must use confirmed network/channel settings, independent account material and actual network commitment roots; stronger protocol-level chain binding remains an integration question.

The descriptor contains public key material, not signing/viewing/nullifier secrets. This implementation proves signing-key control; it does not establish that each member owns a private LEZ identity or has received the shared private state material. The secure distribution, recovery, revocation and private-account authentication protocol remains open. Local diagnostic inputs/outputs can reveal fixture membership and balances and are not privacy-preserving publication artifacts.

Each executed transaction uses two real notes, zero dummy notes and 1024-byte ciphertext padding. Wallet padding, network metadata and coordination privacy have not been measured. Native and guest agreement plus invalid-input checks are developer validation, not an independent audit.
