# Phase 1 — Release Notes

**Version:** 0.1.0 (source preview)

## Implemented
- Tauri 2 desktop project and React + TypeScript interface.
- Read-only Rust discovery engine for native / Flatpak Steam installs.
- Multiple Steam library locations via VDF metadata.
- Game manifests, expected install paths, and expected Proton prefix paths.
- Compatibility tool listings from local Steam directories.
- Search, filters, overview statistics, game detail inspection, rescan, warnings.
- Five Rust unit tests with isolated temporary Steam fixtures.

## Not implemented yet
- Proton crash analysis or automatic collection of Proton logs.
- GPU/Vulkan, driver or dependency diagnostics.
- Game-specific selected Proton version detection.
- Repair, snapshots, install actions, saves, launchers beyond Steam.

## Verification status
- JSON configuration parsing: checked.
- Source/data contract and safety checks: checked.
- Native compilation / UI launch: **not verified** (Rust/Cargo and npm packages unavailable in build environment).

## Next gates
1. `npm install` and `npm run typecheck` on Zorin OS.
2. `cargo test --manifest-path src-tauri/Cargo.toml`.
3. `npm run tauri dev`, then test against real Steam games and mounted libraries.
4. Fix any platform-specific build errors based on command output before advancing to Phase 2.
