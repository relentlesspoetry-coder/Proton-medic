# Proton Medic v0.4.4 — native Linux build

## Fixed

- Corrected Rust ownership handling in Proton Advisor while preserving diagnostic evidence and recommendation order.
- Added `log-unreadable` to the frontend launch-status type.
- Aligned the minimum Rust requirement with the locked Tauri dependencies.
- Used one release codegen unit for the application crate to resolve release object-generation failures.
- Added the DEB dependency on glibc 2.39 or newer.

## Added

- Cargo and npm lockfiles.
- A pinned CI Rust toolchain.
- Regressions for evidence preservation, launch-status contracts, and Rust version checks.
- Reproducible build instructions and a desktop smoke-test script.

The package version is 0.4.4; this version retains the older sidebar label. Later releases update the visible version.

See [TEST-REPORT-v0.4.4.md](TEST-REPORT-v0.4.4.md) for test and packaging results.
