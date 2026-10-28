# Proton Medic v0.4.2 — validation report

Date: 2026-10-28

## Scope

- Continued validation of the v0.4.1 source, rather than adding more features.
- Corrected a Phase 4 launch-observation defect: the analyzer previously picked the newest log across native/Flatpak locations; a different, unchanged, future-dated log could mask a fresh log. It now selects logs whose fingerprint changed **during the current observation** and analyzes that exact path.
- Explicitly marks an unreadable fresh log as `log-unreadable`, instead of `log-no-known-errors`.
- Excludes symlinked log files from this workflow.
- Added two Rust regression tests for mixed Steam log locations and symlinked log rejection. The Rust tests were **written but not run**.
- Original game discovery, auto-selection, repair archive, backup and rollback modules were not modified. The approved interface layout and styling were not modified.

## Tests actually executed in this environment

| Test | Result | What was exercised |
|---|---|---|
| Chromium browser smoke suite | **27 passed / 0 failed** | Actual React TSX components using simulated backend responses, with compatibility adapter |
| Python disposable launcher fixture protocol | **7 passed / 0 failed** | Temporary Steam manifests, fake launcher argv, exit codes, fresh/stale log creation, save-file untouched |
| Node static source/JSON checks | **18 passed / 0 failed** | TSX syntax parsing, configurations, command wiring, safety assertions, preserved module paths |
| Native preflight | **blocked (exit 2)** | Cargo, rustc, native pkg-config libraries, npm dependency directory |
| Rust unit/integration tests | **not run** | Cannot execute without Cargo and dependencies |
| TypeScript typecheck + Vite production build | **not run** | npm dependencies missing |
| Tauri Linux native build | **not run** | Cargo and system packages unavailable |
| Tests against installed Steam games | **not run** | No production Steam installation involved |

### Exact native blocker

Preflight reported missing `cargo`, `rustc`, `gtk+-3.0`, `webkit2gtk-4.1`, `javascriptcoregtk-4.1`, `libsoup-3.0` and project `node_modules`. A registry probe also failed due to DNS, so dependencies could not be fetched. No native executable or installer was produced.

## How to finish native validation

On Zorin OS, install Node.js, Rust/Cargo and the official Tauri 2 Linux development dependencies, extract this archive, then:

```bash
npm install
./scripts/validate-native.sh
```

On success the script runs Rust tests, TypeScript build, and Tauri packaging. First test writes and repairs only disposable Steam-style directories; do not use important real game saves until native tests and on-machine trials pass.

## Integrity

Previous version retained separately. All required Phase 1–4 source and project files included. Original UI stylesheet, main view, scanner, repair, advisor and auto-select source files byte-identical to v0.4.1.
