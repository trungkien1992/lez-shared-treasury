# Third-party notices

`testdata/bip340-test-vectors.csv` is an unchanged copy of the Bitcoin BIP-340 published vectors, retrieved from `bitcoin/bips`, Git blob `672339129a844a060591bb22f444158ff45438ed`.

Source: https://github.com/bitcoin/bips/blob/master/bip-0340/test-vectors.csv

BIP-340 authors: Pieter Wuille, Jonas Nick and Tim Ruffing. The BIP identifies its specification license as BSD-2-Clause, and its code license as BSD-2-Clause OR MIT OR CC0-1.0. This redistribution elects CC0-1.0 for the accompanying code/test-vector material; see https://creativecommons.org/publicdomain/zero/1.0/ . Attribution is retained here. The project runs the 15 published 32-byte-message vectors matching the inspected LEZ prehashed API; variable-length-message vectors are preserved but not run.

Rust dependencies are resolved from the included lockfiles and retain their own copyright and license terms. They are not vendored in this archive. Official compiler and proving-runtime binaries are also not redistributed.

The archive includes the exact compiled guest program. Available license and notice files for its resolved normal dependency tree are retained in `third-party-licenses/`, with package versions, SPDX declarations and repositories in `inventory.json`. This includes dependencies selected by Cargo even when linker elimination may remove their code. Crates declaring Apache-2.0 without a separate packaged license file refer to the full `LICENSE-APACHE` text. The `include_bytes_aligned` package supplies an MIT declaration without a standalone notice file; its exact manifest declaration and standard MIT text are retained. Original upstream files are not reattributed to this project.

The accelerated guest additionally uses the official RISC Zero forks of k256, sha2 and crypto-bigint, pinned in the guest lockfile, plus risc0-bigint2 1.4.14. Their declarations and available notices are included separately from the baseline versions. The inventory retains the union of the baseline and accelerated dependency notices, including packages selected across Cargo target configurations.
