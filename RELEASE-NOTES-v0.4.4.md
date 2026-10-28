# Proton Medic 0.4.4

This release focuses on native Linux compilation and executable verification.
Consult TEST-REPORT-v0.4.4.md for the final packaging and runtime outcome.

## Changes

- Fix Rust E0505 in Proton Advisor: evaluate whether diagnostic evidence
  indicates a system problem before transferring evidence into the result.
  No evidence cloning or changes to recommendation precedence are needed.
- Fix TypeScript TS2367 by including the backend's `log-unreadable` status in
  the frontend LaunchResult contract.
- Add regressions for system-error evidence preservation, frontend/backend
  launch-status parity, and the Rust version preflight boundary.
- Add Cargo and npm lockfiles, use locked Cargo operations and npm CI, and
  pin the CI Rust version to the toolchain used for this validation.
- Require Rust >=1.90 in the manifest and preflight, matching locked Tauri.
- Configure one release codegen unit for the application crate following
  repeated empty-object failures with default parallel code generation.
- Add reproducible build instructions and an optional native GUI smoke
  capture script with an isolated disposable Steam library.

## Preserved behavior

The UI components and styles are unchanged. Steam discovery, diagnostics,
Proton auto-selection, game launch observation, repairs, backups, and rollback
modules are retained. Only Advisor ownership handling and a frontend type
union change application source behavior at compile time; the recommendation
logic and safety/consent checks are preserved.

The sidebar's existing hardcoded `v0.4.0` label is retained with the supplied
UI; package and executable metadata identify this build as 0.4.4. This cosmetic
label should be made metadata-driven in a subsequent UI maintenance change.

The original 0.4.3 archive is included unchanged in the delivery backup.
