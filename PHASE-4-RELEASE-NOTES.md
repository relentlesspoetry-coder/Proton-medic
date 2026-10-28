# Proton Medic v0.4.0 — launch observation

## Added

- **Launch & Check Game** in Steam game details.
- Validation of the scanned App ID, library, and install folder before launching.
- Native Steam and Flatpak Steam launch commands with separate process arguments.
- A ten-second UI observation window, with a backend range of 5–45 seconds.
- Fresh-log analysis with separate launcher-error, log-error, warning, clean-log, and no-log states.
- A player confirmation step for whether the game works.
- Temporary fake-Steam launcher tests and a native validation script.

The launcher leaves Steam and game processes running after observation. Its exit status describes the launch request, while new Proton logs supply diagnostic findings. Logging must be enabled to collect a new log.

Existing discovery, diagnostics, recommendations, Proton selection, repairs, and rollback remain available.

See [PHASE-4-TEST-REPORT.md](PHASE-4-TEST-REPORT.md) for this version's test results.
