# Proton Medic v0.4.1 — launch test reliability

## Fixed

- Added the missing `configured_tool` field to a repair test fixture.
- Analyzed launch logs using the selected temporary home directory.
- Corrected the expected result for Vulkan device-loss evidence.

## Added

- Rust tests for device-loss findings and fresh logs without recognized errors.
- Seven offline fake-Steam protocol tests.
- A combined native validation command and GitHub Actions configuration.

See [PHASE-4.1-TEST-REPORT.md](PHASE-4.1-TEST-REPORT.md) for test results.
