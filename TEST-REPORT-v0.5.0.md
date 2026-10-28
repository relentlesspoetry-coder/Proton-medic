# Proton Medic v0.5.0 — Phase 5A verification

Date: 28 October 2026

## Outcome

The new Fix & Test workflow was implemented and the actual Rust/Tauri app was
compiled in test and optimized release modes. The complete release gate passed,
and the amd64 DEB was installed successfully in the disposable Ubuntu container.
This is a development milestone, not a claim that every Linux game is repairable.

## Executed results

| Verification | Result |
| --- | --- |
| Rust tests (`cargo test --locked`) | 56 passed, 0 failed/ignored |
| Existing Python disposable launcher fixtures | 7 passed |
| Python build-gate regressions | 5 passed |
| Frontend/native status contract | 1 passed |
| New React interaction tests (Vitest + jsdom) | 5 passed |
| TypeScript syntax, JSON and structural checks | 19 passed |
| Strict TypeScript typecheck + Vite production build | Passed |
| Full `scripts/build-zorin.sh --build` | Exit 0 |
| Tauri optimized Linux build + DEB bundling | Passed |
| DEB metadata inspection and `dpkg -i` | Passed: install ok installed 0.5.0 |
| Installed executable compared with extracted DEB payload | Exact match |
| `ldd` shared-library resolution | No missing libraries |
| Native GUI and real Steam/gameplay end-to-end | Not verified in this phase |
| Cross-distro, SteamOS, controller and Wayland/X11 hardware | Not executed |

Total: **74 executable tests plus 19 structural checks**. The Rust total includes
all 44 previous tests and 12 new regressions. Frontend interaction tests use
mocked Tauri IPC in jsdom; they are not native WebKit or real-game tests.

## What the new tests cover

- Eligibility only for fresh, supported exception/device-loss findings with an
  installed alternative; rejects stale/future/missing evidence, missing game
  installations, system prerequisites, and explicit anti-cheat blockers.
- Ordinary anti-cheat references do not become unsupported-platform claims.
  The existing automatic advisor also refuses explicit anti-cheat blockers.
- Every workflow mutation requires approval before scanning, writing, or launch.
- A failed launch consumes the single attempt and is not automatically retried.
- Rollback preserves the original backup and separately backs up the displaced
  configuration; rejects symlink targets, changed target mappings, and invalid
  data. Unrelated Steam edits are preserved. Newly inserted game overrides are
  removed without removing other games' mappings.
- UI approval gating; no launch after a failed change; rollback access and backup
  display after launch failure and component remount; clean logs require human
  playability confirmation; unsupported findings have no apply control.

Existing native tests still exercise disposable Steam discovery, actual fake
launcher subprocesses, diagnostic log handling, atomic configuration changes,
archive backups, and prefix/cache restoration. No real Steam library or game
saves were used. The new full workflow was not exercised through live Tauri IPC
against a running Steam client; this remains a target-machine acceptance test.

## Implementation scope

The new game-details section coordinates a single Proton-version repair using
existing native discovery, advisor, backed-up selection and launch observation.
It revalidates the plan/candidate before writing and requires Steam closed for
configuration changes and rollback. It never applies shell commands from logs,
deletes prefixes, downloads arbitrary runtimes, or claims an observed launch is
proof of playable gameplay.

Only one recipe is implemented here: an installed Proton alternative for fresh
Wine exception/Vulkan device-loss evidence. Dependency installation, standalone
Windows installers, managed runtime downloads, store integration and native-game
repair recipes remain future work. Read PHASE-5-RELEASE-NOTES.md for the roadmap.

Rollback session receipts live in backend memory and UI sessionStorage. Keep the
app open until keeping or rolling back a fix. Backups persist after app exit,
but automatic recovery of a receipt after restart is not implemented. If this
game's mapping changes after the fix, automatic rollback refuses to overwrite
it. It preserves unrelated configuration edits and does not stop Steam or games.

## Environment and packaging

Ubuntu 24.04.3 LTS x86_64 disposable build container; workspace-local Rust/Cargo
1.99.1; Node.js 24.22.1; npm 11.9.0; GTK 3.24.41; WebKitGTK 2.52.3; libsoup 3.4.4.
Tauri 2.12.5. Locked npm/Cargo dependencies are included. Application release
codegen remains one unit, retaining the previously verified build workaround.

The DEB requires libc6 >=2.39, GTK 3 and WebKitGTK 4.1. It is not a universal Linux
or SteamOS package. Broad-distro Flatpak/AppImage builds remain pending. The
previous phase demonstrated sandbox X display/D-Bus restrictions; this phase did
not claim or repeat a successful graphical startup. No current UI screenshot is
provided; old screenshots/reports in the source are historical only.

## Preservation

The charcoal/green stylesheet is byte-identical to v0.4.4. Existing diagnostics,
advisor, repair and launch components remain, with a new section added to game
details and an accurate sidebar version. Scanner, launch observation, and prefix/
cache repair modules are unchanged. Existing auto-selection is retained and
extended with tracked rollback; explicit anti-cheat blocking is added to advice.

The v0.4.4 complete delivery is preserved byte-for-byte in the new backup. It
also includes the original v0.4.3 backup.

Previous delivery SHA-256:
`e31ff80d71d10ade76d04e7b01c4a677f53a7a5cc80fe8bb0f7d4d3ffc756016`

v0.5.0 DEB SHA-256:
`02409f20b76db6999420f0970ec165c2c652c0cc2b5fbf6996f2edb4c02b53e7`

Evidence: `phase5-evidence/full-release-gate.log`, `frontend-build.log`,
`package-info.log`, `install.log`, `package-verified.log`, `ldd.log`, and
`preservation.log`. The gate also lists the prior cached 0.4.4 package; the new
0.5.0 artifact was checked and installed independently. No remote CI run is claimed.

## Target-machine acceptance still needed

On a compatible Linux desktop, use a disposable game: diagnose a known failure,
approve Fix & Test with Steam closed, inspect the actual game and fresh log,
then close Steam and test rollback. Verify that unrelated settings and save data
remain intact. Also check navigation during the session, launch failure, and
app-exit recovery limitations. Do not equate these pending tests with the
executed unit and component results above.
