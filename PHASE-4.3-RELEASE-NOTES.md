# Proton Medic v0.4.3 — native build gate

Source release.

## Added

- `scripts/build-zorin.sh --check` checks build tools, native libraries, and the Rust version.
- `scripts/build-zorin.sh --build` runs tests and builds a DEB after prerequisites pass.
- Build-gate tests cover preflight status, error handling, and source preservation.
- GitHub Actions configuration builds a DEB after validation.
- `scripts/validate-native.sh` remains available as a compatibility entry point.

See [PHASE-4.3-TEST-REPORT.md](PHASE-4.3-TEST-REPORT.md) for test results.
