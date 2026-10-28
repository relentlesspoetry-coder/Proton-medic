# Proton Medic v0.4.3 — Zorin native-build gate

**Scope:** Build readiness, not a new feature release. The previously approved
React UI, the scanner, diagnostic engine, auto-selection, repairs, backups,
rollback and opt-in game launch tests remain unchanged.

## Added

- `scripts/build-zorin.sh --check`: read-only prerequisites and Rust >=1.82 check.
- `scripts/build-zorin.sh --build`: fail closed unless prerequisites are present;
  then run source checks, disposable fixtures, Cargo tests, frontend typecheck
  and production bundle, and a Tauri `.deb` build.
- `scripts/test-build-gate.py`: tests the preflight interface, error handling,
  source immutability and compatibility wrapper.
- GitHub Actions workflow updated to gate native dependencies and build a
  `.deb` only. This workflow has **not** been executed from this environment.
- Retained `scripts/validate-native.sh` as the older command entry point.

## Not included

- A compiled `.deb` or `AppImage` binary.
- Passed Rust/Cargo test results.
- Real Steam filesystem or game-run validation.
- Automatic unattended cycling through Proton versions.

This runtime does not have Cargo/rustc, WebKitGTK/GTK development libraries,
or downloaded npm dependencies. Do not run the unverified repair backend on
important game saves until the native build and disposable Rust tests pass.
