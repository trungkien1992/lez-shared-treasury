# Member client decisions — 13 October 2026

| Decision | Choice and reason | Trade-off |
| --- | --- | --- |
| First transport | Offline canonical request and response files, with a transport-neutral Rust API | Useful for testing independent signers now; Logos Chat/E2EE and room identity binding remain unimplemented |
| Instruction representation | Borsh types shared directly with the archived guest, rather than a second JSON interpretation of payment amounts | No alternate description to trust; currently only the archived native-transfer and creation instructions are supported |
| Review confirmation | Require the digest of the exact reviewed bytes when signing | Forces a new review after any byte change; cannot guarantee the human actually reads the display |
| Enrollment policy | Preserve the guest's unanimous descriptor enrollment | Every initial member consents; one unavailable member can prevent creation |
| Rejections | Domain-separated signed opinions, without a veto | Matches M-of-N approval semantics; a member's conflicting responses halt the client collection until a new request |
| Duplicate delivery | Verify each response and count each member once; canonicalize response ordering | Tolerates retries and reordered transport; withholding remains an availability problem |
| Signing-key storage | OS-generated keys in restrictive local development files, with no secret-key transport to the collector | Hardware wallets and an encrypted production keystore remain work for a later module |
| Chain state | Require an independently obtained local context; export a clearly labelled predicted next context | The client cannot authenticate or synchronize a live private vault note yet |
| Guest compatibility | Preserve the exact guest and all shared core source; add host-only import points | Existing proof evidence remains relevant; the local bridge still uses fixed financial test parameters |

The current RFP's byte-derived review and asynchronous approval requirements guided this milestone. Reference: https://logos.co/builders-hub/rfps/multisig-and-coordination (checked 13 October 2026). No public proposal text, audit appointment, or submission was changed by this work.
