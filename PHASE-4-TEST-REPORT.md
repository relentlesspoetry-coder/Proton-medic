# Proton Medic v0.4.0 — Verification Report

Date: 2026-10-28

## Results actually executed here

| Check | Result | Evidence |
|---|---|---|
| Chromium browser UI regression suite (simulated Tauri responses) | **PASS — 27/27** | `UI-TEST-RESULTS.json`, `ui-test-v0.4.0-run.log` |
| TypeScript/TSX syntax + frontend/backend command contract + safety structure | **PASS — 18/18** | `scripts/check-source.cjs` |
| `Cargo.toml` TOML parser | PASS | Python 3 `tomllib` parsed package version `0.4.0` |
| Native validation script shell syntax | PASS | `bash -n scripts/validate-native.sh` |
| Native preflight | **BLOCKED (exit 2)** | `native-validation-output.txt` |
| Rust compilation and Rust unit tests | **NOT EXECUTED** | Cargo / Rust compiler unavailable |
| Full TypeScript typecheck and production frontend build | **NOT EXECUTED** | Missing installed npm dependencies / registry unreachable |
| Native Tauri package (`.deb`, `.AppImage`) | **NOT EXECUTED** | Native dependencies missing |
| Real Steam/Flatpak launch | **NOT EXECUTED** | No local Steam installation or Linux desktop in sandbox |

## New browser tests executed

1. Launch button remains disabled until the user approves opening a game.
2. Launch command uses selected App ID, library path, 10-second observation, and explicit `confirmed: true`.
3. No fresh Proton log is presented as inconclusive, not success.
4. Player must independently confirm playability.
5. Fresh-log errors are shown as warnings, not a success verdict.
6. Backend launch failures are displayed clearly.

All earlier scanner, diagnostics, advisor, auto-selection, repair, rollback, system-check, search, and settings UI regressions continued to pass.

## Rust disposable fixture tests added but not executed

`src-tauri/src/launch_test.rs` contains **six `#[test]` functions**:

- Error classification never treats a clean log or launcher exit as game success.
- An unchanged existing Proton log is not fresh evidence.
- A disposable shell `fake-steam` launcher is invoked using a temporary directory rather than real Steam files.
- The fake launcher emits a new synthetic Proton log; it is still not marked automatically playable.
- A fake launcher returning a nonzero exit code is classified as an error.
- User confirmation and a bounded observation duration are enforced before any launch.

These tests are part of the Rust module and require Cargo to execute. **No claim of these six tests passing is made.**

## Native preflight blocker

The sandbox does not contain Cargo or rustc and lacks `gtk+-3.0`, `webkit2gtk-4.1`, `javascriptcoregtk-4.1` and `libsoup-3.0` development packages. The npm registry is unreachable, so JavaScript dependencies cannot be installed here. `scripts/validate-native.sh` deliberately exits before attempting any compilation or game launch when prerequisites are missing.

## How to complete native verification

On Zorin OS, install official Tauri 2 Linux prerequisites and Rust/Cargo, then run:

```bash
npm install
./scripts/validate-native.sh
```

Once this passes, separately run the app in an actual Steam environment, using a disposable test game before enabling auto selection/repair on important games. Steam's command-line launcher exit does not prove the game ran correctly. Proton Medic observes logs conservatively and asks for confirmation of playability.
