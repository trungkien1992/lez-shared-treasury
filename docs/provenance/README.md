# Source and evidence provenance

This release was prepared from version 4 of
`LEZ_Shared_Treasury_Real_Lifecycle_2026-10-12.zip`.
Despite its filename, that archive contains the 17 October client milestone.
Its SHA-256 is
`1e391e30854cd63d62e5e7dc3a5ddbacc7cf274566f6ec63f8eb60882bfebab6`.
Its 1,308-entry file manifest was checked before preparing this release.

`unchanged-source.json` records the byte identities of every Rust source,
Cargo manifest/lockfile and retained lifecycle guest image. All 31 Rust files
retain the snapshot bytes. Reviewer documentation and release scripts were added;
the member-client README and CI recipe were clarified for the new layout.

`full-archive-manifest.json`, `status-2026-10-17.json` and
`readme-2026-10-17.txt` are historical reference files. The original full-archive
manifest intentionally lists files omitted here; it is not the manifest for this
release. `evidence/MANIFEST.json` covers the current release and is checked by
`scripts/verify_evidence.py`.

The omission list contains compiled Linux host/client executables, demonstration
media and large intermediate proving receipts. Final succinct receipts,
transaction encodings, state fixtures, public test responses, source and notices
remain available. Fresh proving can generate new intermediate receipts; old
intermediate-stage checkpoint resume files are not distributed here.

The historical private-account/nullifier fixture material is public and
deterministic. Fresh member signing secrets from the 13/17 October demos were
excluded from the original archive. No private commercial proposal, Telegram
transcript, provider document URL, access token or contractor correspondence is
included in this evidence release.
