# Proton Medic v0.4.2 — fresh launch logs

## Fixed

- Select logs changed during the current launch observation, preventing an unrelated newer timestamp from masking the new log.
- Report unreadable fresh logs as `log-unreadable`.
- Exclude symlinked log files from launch observation.

## Added

Rust regressions for mixed native/Flatpak log locations and symlink rejection.

See [PHASE-4.2-TEST-REPORT.md](PHASE-4.2-TEST-REPORT.md) for test results.
