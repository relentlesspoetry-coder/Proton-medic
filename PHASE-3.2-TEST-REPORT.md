# Proton Medic 0.3.2 — test report

## Browser UI tests

`python ui-sandbox/test_ui.py` — **22 passed / 0 failed**.

Includes preserved Phase 1–3 UI flows and these additional Phase 3.2 cases:
- Per-game auto-select action invokes `auto_select_proton` with the scanned game ID and explicit authorization.
- An opt-in Settings switch enables and disables automatic changes.
- When enabled, a fresh scan with recent compatibility errors triggers a version selection request.
- Vulkan driver errors are displayed as System Check issues instead of a Proton-switch suggestion.

These smoke tests run actual React screen components inside a Chromium adapter with **simulated backend results**. They do not test real Steam, Rust, or Tauri.

## Source verification

- `node ui-sandbox/build.cjs`: TypeScript/TSX syntax parsing and transpilation passed.
- ZIP archive integrity and expected native/React files verified.
- Rust unit tests written for minimal VDF editing, backup/writes, and custom tool ID resolution; **not executed**.
- Native Rust / GTK / WebKit / Tauri build: **not executed**, build toolchain unavailable in this environment.

## Safety warning

Do not activate write-mode against important Steam configurations until the Rust test suite and native build have passed on Linux. The selected tool is a test candidate; game launch verification is needed to establish compatibility. Automatic per-game rollback is not implemented yet; timestamped config backups can be restored manually with Steam fully closed.
