# Proton Medic v0.5.1 — automatic log reader validation

Date: 28 October 2026

## Delivered behavior

Automatic polling of standard Steam Proton logs every ten seconds while the
application is open. Metadata is checked before bounded analysis of changed
logs. Startup logs form a read-only baseline. New evidence is analyzed, allowed
to settle for a poll, and classified using the existing diagnostic/advisor rules.

With the new opt-in automatic-fix setting enabled, a supported fresh finding is
queued while Steam runs and then sent to the existing backed-up Fix & Test
writer after Steam closes. The writer rescans/revalidates before applying.
No further per-change click is required. The UI displays the outcome and backup
path and delivers the session receipt to Fix & Test for launch approval or
rollback. Existing manual selection and repair controls remain available.

Only the previously implemented installed-Proton alternative recipe is applied.
This is not generalized DLL/package installation, runtime downloading, prefix
resetting, standalone-game log parsing, or an anti-cheat bypass. Logs must exist;
the UI explains `PROTON_LOG=1 %command%`. Custom log paths are not supported yet.

## Executed checks

| Check | Result |
| --- | --- |
| Actual native Rust tests | 60 passed; 0 failed/ignored |
| Python disposable launcher tests | 7 passed |
| Python build gate tests | 5 passed |
| Frontend/native launch-status contract | 1 passed |
| React/Vitest/jsdom interaction tests | 9 passed (5 existing + 4 new) |
| Syntax/config/structural checks | 20 passed |
| Strict TypeScript + Vite production build | Passed |
| Full native release gate | Exit 0 |
| Optimized Tauri compilation and DEB bundling | Passed |
| Final DEB metadata and installation | Passed: install ok installed 0.5.1 |
| Installed executable vs extracted package payload | Exact byte match |
| Shared libraries (`ldd`) | No missing libraries |
| Real desktop/gameplay/end-to-end automatic repair | Not executed |
| Cross-distro/SteamOS validation | Not executed |

Total: 82 executable tests plus 20 structural checks. Frontend interaction tests
mock Tauri IPC; they do not demonstrate native WebKit or a running Steam client.
The previous environment's X display/D-Bus restrictions remain a GUI validation
limitation. No successful native GUI launch or real-game repair is claimed here.

## New regression coverage

Rust: startup baseline and read-only mode cannot authorize a write; growing logs
must settle; queued work waits for Steam closure; an attempted game is not
rearmed by more log writes; disposable file metadata detects a new/growing log
and ignores symlinks. Existing fixture tests still exercise real configuration
writes, backup preservation, selective rollback, and fake launcher subprocesses.
The new poll-to-writer path has not been exercised end-to-end against real Steam.

React: polling starts without a manual scan; long-running requests do not
schedule overlapping polls; explicit toggle action is required for enabling
writes; polling stops on unmount; rollback receipt delivery preserves an existing
launch-attempt flag and does not repeatedly overwrite the receipt.

## Safety and operational boundaries

- New opt-in preference defaults off and does not silently reuse the old
  scan-triggered automatic-selection consent. The manual selection button remains.
- The old automatic effect is replaced so it does not compete with the log monitor.
- One automatic attempt per game per app session; failures also consume the
  attempt. At most one candidate is attempted per polling cycle.
- A settled log is not assumed to mean the game stopped. Native Steam-closed,
  fresh evidence, current recommendation and atomic configuration guards still run.
- Disabling prevents future requests; an already-authorized operation in flight
  may finish. Read-only monitoring continues.
- Games do not launch automatically. User approval remains required for testing.
- Explicit anti-cheat/system blockers do not receive this Proton-switch recipe.
- Keep the app open to retain automatic rollback receipts. Disk backups survive
  exit; persistent recovery of session receipts is not implemented yet.
- Metadata-based monitoring cannot detect a same-size rewrite that deliberately
  preserves every observed modification timestamp.

## Environment and package

Disposable Ubuntu 24.04.3 x86_64 container; workspace-local Rust/Cargo 1.99.1;
Node.js 24.22.1; npm 11.9.0; Tauri 2.12.5; GTK 3.24.41; WebKitGTK 2.52.3.
The DEB requires glibc >=2.39. This is not yet a universal Linux/SteamOS installer.
AppImage/Flatpak builds and real hardware validation remain pending.

The gate lists older cached DEBs as well; the new 0.5.1 DEB was checked and
installed independently. No remote CI execution is claimed.

Installer SHA-256:
`40e2be44dbe70d8a5f2cab9ac4f6eabaa154ff250205ed513c07976ac23d9c3d`

Prior v0.5.0 complete-delivery backup SHA-256:
`a04197deefdac3e2bae4cb2a9bd040a33e94041e08899f44d8b80747ee557809`

## Evidence and preservation

See `phase51-evidence/release-gate.log`, `frontend-tests.log`, `rust-tests.log`,
`frontend-build.log`, `package-info.log`, `install.log`, `package-verified.log`,
and `ldd.log`. The full previous v0.5.0 delivery is backed up inside this delivery,
including the earlier backups. The existing stylesheet, scanner, launch test and
prefix/cache repair implementations remain unchanged. UI additions use the
existing styling. Historical reports and screenshots are not current test proof.

Target-machine acceptance still needs a disposable Steam game producing a new
failure log: verify observation, Steam-open deferral, opt-in application after
Steam closes, correct backup/receipt, explicit launch testing, and rollback.
