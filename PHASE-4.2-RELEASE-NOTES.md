# Proton Medic v0.4.2

**Native validation readiness and launch log reliability update.**

- Fixes choosing a stale log instead of the fresh log produced by the current Steam launch probe.
- Prevents unreadable fresh logs from being presented as having no recognized errors.
- Adds disposable Rust regression tests for mixed native/Flatpak log locations and symlink rejection.
- Preserves all earlier features and approved UI.
- No native build or Rust unit test execution was possible in the current environment; see test report.
