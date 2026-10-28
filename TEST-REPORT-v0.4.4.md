# Proton Medic v0.4.4 — native validation report

Date: 28 October 2026

## Outcome

**Actual native Rust/Tauri compilation, automated tests, release compilation,
DEB packaging, and package installation succeeded. GUI startup and real Steam
Deck/gameplay behavior are not verified.** The desktop smoke attempt was
blocked by this environment's display/socket restrictions.

The complete source, original backup, evidence logs, and installer are supplied
in the delivery archive. This is an Ubuntu 24.04 x86_64 build, not a SteamOS
installer or a claim of compatibility with every Ubuntu/Zorin version.

## Executed checks

| Check | Result |
| --- | --- |
| Native prerequisite gate | Passed |
| Existing TypeScript syntax/config/structural checks | 18 passed |
| New frontend/native launch-status contract test | 1 passed |
| Contract regression against original v0.4.3 types | Failed as expected; fixed types restored |
| Python disposable-game/launcher fixture tests | 7 passed |
| Python native-build-gate tests | 5 passed (4 existing + 1 new) |
| Actual Cargo native unit tests | 44 passed (43 existing + 1 new), 0 failed/ignored |
| Binary and documentation test targets | Executed, 0 tests in each |
| TypeScript strict typecheck and Vite production bundle | Passed |
| Actual Tauri optimized application build | Passed |
| Full release gate after compiler fixes | Exit 0; tests, frontend, native build and DEB check passed |
| Final installer metadata rebundle | Passed; adds explicit libc6 >=2.39 dependency |
| Final `dpkg -i` installation | Passed; `install ok installed 0.4.4` |
| Installed executable versus extracted final DEB payload | Byte-for-byte comparison passed |
| Shared-library resolution (`ldd`) | No missing libraries in build environment |
| Native GUI smoke under Xvfb/D-Bus | Blocked; not passed |
| Real Steam games, GPU/Vulkan gameplay, Steam Deck/SteamOS | Not executed |
| Remote GitHub Actions | Not executed |

There are 57 executable unit/fixture/contract tests, plus 18 structural checks.
Python fixtures are independent protocol checks and are not represented as
Rust tests. Existing historical Chromium/UI reports were not rerun or counted
as current validation.

## Environment

- Disposable Ubuntu 24.04.3 LTS container, x86_64; no host Steam library mounted.
- Workspace-local Rust/Cargo 1.99.1; Node.js 24.22.1, npm 11.9.0.
- Tauri 2.12.5, Tauri build 2.7.4, locked dependency resolution.
- GTK 3.24.41, WebKitGTK/JavaScriptCore 2.52.3, libsoup 3.4.4.
- GCC 13.3.0; glibc 2.39. The executable references GLIBC_2.39.
- Native development dependencies installed inside the disposable container.
  APT required a workspace-local archive cache and disabled `_apt` privilege
  dropping because that user transition is unsupported here. No host packages
  or real Steam data were changed.

## Failures found and corrected

1. **Rust E0505, `src-tauri/src/advisor.rs`:** a closure borrowed `evidence`
   after the vector was moved into `VersionAdvice`. Compute the system-error
   boolean before the move. New Rust coverage verifies both Vulkan and
   missing-library evidence is preserved, with no Proton switch suggested.
2. **TypeScript TS2367, `src/LaunchTest.tsx`:** backend and component handle
   `log-unreadable`, but `LaunchResult.status` omitted it. Add the missing
   union value. New contract test compares emitted Rust statuses with the
   TypeScript AST and demonstrably fails on the original type definition.
3. **Preflight minimum mismatch:** resolved Tauri requires Rust 1.90, but the
   original manifest/preflight accepted 1.82. Align both to 1.90; test rejection
   at 1.89 and acceptance at 1.90. Only 1.99.1 was used for actual compilation.
4. **Release object generation:** two attempts with default parallel codegen
   produced an empty object and failed static-library assembly with
   `memory map must have a non-zero length`. Cleaning application artifacts
   and limiting Cargo to one job alone did not fix it. One release codegen
   unit for the application crate produced successful optimized builds.
   The underlying compiler/environment cause is not established. All three
   original crate outputs (staticlib, cdylib, rlib) remain enabled.
5. **Installer minimum runtime:** add `libc6 (>= 2.39)` to DEB dependencies;
   Tauri adds GTK/WebKit dependencies. This packaging-only change was
   rebundled using Tauri's already-built application command and installed
   again successfully. It was not represented as a new compilation.

Cargo.lock/package-lock.json are included; Cargo release/test commands use
`--locked`, and CI uses `npm ci` and Rust 1.99.1. The new test runs in both the
local gate and CI definition.

## Runtime limitations and failed attempts

`dbus-run-session` failed to open its socket (`Operation not permitted`).
Running without D-Bus reached the executable but GTK initialization failed.
An independent `xdpyinfo`/`xdotool` probe also could not open the Xvfb display,
so a functional desktop was unavailable. No screenshot or successful window
render is claimed, and no application sandbox setting was disabled.

An intermediate check comparing the installed executable to the bundler's
working binary failed. The final verification compares the installed file
with the exact final DEB payload instead and passed. Logs retain both attempts.

Desktop launch, controller/touch interaction, Wayland, real GPU drivers,
actual Steam launches, and real-save backup/restore still need target-machine
validation. The DEB is not a SteamOS package. Older glibc systems require a
separate compatible build; this installer now declares its minimum.

The preserved UI includes its historical hardcoded sidebar label `v0.4.0`;
actual package metadata is 0.4.4. No UI redesign was performed.

## Preservation and evidence

The approved UI components/styles, scanner, diagnostics, automatic selection,
launch testing, repair and rollback modules were byte-compared with the input
archive and remain unchanged. Advisor's ownership fix and the frontend status
type are the application-source changes. No features or consent checks were
removed, and no destructive automatic repairs were added.

Original archive SHA-256:
`646656c57cd4c30b525c8d0d231a21f30a70867230521c7505ee6f87cdafbc27`

Final DEB SHA-256:
`c0cf3c8dc6d85e55c8ea09c1cf0d0714ba1cfa74d48113d58b667522d91c2288`

Key evidence files in the delivery's `evidence/` directory:
`initial-frontend-fixtures.log`, `regression-original.log`,
`frontend-tests.log`, `final-native-gate-tail.log`, `release-clean-retry.log`,
`release-success.log`, `final-rebundle.log`, `final-install.log`,
`final-package-verification.log`, `package-install.log`,
`native-smoke.log`, `native-smoke-without-dbus.log`, `display-probe.log`, and
`preservation-check.log`. Early logs include failed or partial attempts;
the explicit final results above supersede older historical reports.

## Install on a compatible Ubuntu 24.04 desktop

```sh
sudo apt install ./proton-medic_0.4.4_amd64.deb
proton-medic
```

Run as the normal desktop user. The optional `scripts/smoke-native.py` can
capture a native window using a temporary Steam fixture on an environment
with a working display, xdotool, and Pillow. See BUILDING.md to reproduce.
