# Proton Medic v0.4.1 — Phase 4.1 Validation Report

Date: 2026-10-28

## Result summary

| Stage | Result | Actual execution |
| --- | --- | --- |
| Chromium browser user-interface regression | PASS: 27/27 | Ran against UI sandbox with simulated backend |
| Dependency-free source/TypeScript syntax checks | PASS: 18/18 | Ran against updated source |
| Disposable Steam launcher/file protocol tests | PASS: 7/7 | Executed actual fake shell launchers and temporary filesystem fixtures; **not the Rust engine** |
| Native prerequisite preflight | BLOCKED | Ran; Cargo/rustc and GTK/WebKit native libraries absent; npm node_modules absent |
| Rust unit tests, including enhanced launch fixtures | NOT RUN | No Rust toolchain, crate registry inaccessible |
| Native Tauri compilation/package | NOT RUN | Rust/Tauri build prerequisites unavailable |
| Real Steam/Proton game test | NOT RUN | No real Steam installation involved |

## Source fixes

1. **Rust compile blocker fixed in source:** `repairs.rs` test-only `Game` constructor was missing the `configured_tool` field introduced in Phase 3.2.
2. **Disposable log consistency:** `launch_test.rs` now invokes `diagnostics::analyze_game(home, ...)` for the exact test home directory rather than diagnosing only the process-global `$HOME`. This supports actual fixture-backed log-signature tests and avoids observing one log while analyzing another home directory.
3. **Outdated test corrected:** The pre-existing `vk_error_device_lost` fixture expected `log-no-known-errors` because it hadn't been parsed. It now expects an error and verifies the `vulkan-device-lost` signature.
4. **New Rust tests written (not run):** New device-loss signature test and fresh-clean-log inconclusive result test.
5. **Offline fixture executable checks:** `scripts/test-disposable-fixtures.py` provides seven true subprocess/file fixture tests isolated in `tempfile.TemporaryDirectory`.
6. **Validation automation:** `scripts/validate-native.sh` executes always-available tests and then checks native prerequisites; optional GitHub Actions workflow is included but **has not run**.

## Test commands executed

```
node scripts/check-source.cjs
python3 scripts/test-disposable-fixtures.py
python3 ui-sandbox/test_ui.py
./scripts/validate-native.sh
```

Last command exits `2` by design when native prerequisites are missing, after running static and disposable protocol checks. See `NATIVE-VALIDATION-OUTPUT.txt` for exact errors.

## Safety and remaining validation

The disposable tests never open real Steam, never modify game directories outside the temporary directory, and never intentionally delete game saves. Existing repair and backup functionality was preserved in source; backend repair/rollback Rust tests remain unexecuted in this workspace. A successful native build and real disposable Steam fixture integration must precede production use of automatic settings changes or repair operations. No installed executable is included in this ZIP.

To finish validation on Zorin OS: install official Tauri 2 dependencies and Rust, run `npm install`, then execute `./scripts/validate-native.sh`. The same checks are configured in `.github/workflows/linux-validation.yml` for a future repository run; this workflow is not an executed test result.
