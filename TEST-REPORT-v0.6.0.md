# Proton Medic 0.6.0 — Windows installer verification report

Date: 28 October 2026

## Outcome

Implemented a standalone Windows installer and game-launch flow. The actual
Rust/Tauri application compiled, the full automated release gate passed, and
an amd64 DEB was produced. Windows execution remains unverified: a compiled
Windows console fixture could not start because this container prohibits
Wine's required socket. No real graphical installer or commercial game was run.

## Executed checks

| Check | Actual result |
| --- | --- |
| Rust unit and filesystem/subprocess fixture tests | 80 passed; 0 failed; 0 ignored |
| React/Vitest/jsdom tests | 18 passed, including 6 Windows Games cases |
| Node frontend/native launch-status contract | 1 passed |
| Python disposable-game fixtures | 7 passed |
| Python native build-gate regression tests | 5 passed |
| Source syntax/config/structural checks | 21 passed |
| Strict TypeScript and Vite production build | Passed |
| Full native release gate | Passed |
| Optimized Rust/Tauri executable and DEB build | Passed |
| MinGW Windows console fixture compilation | Passed; PE executable produced |
| Real Wine fixture execution | Failed before Windows execution: `wineserver: socket: Operation not permitted` |
| Native GUI, native file dialog, graphical installer and actual game | Not verified |
| SteamOS, other distributions, real native Proton installer | Not verified |

Total: **111 automated tests plus 21 structural checks**. These totals exclude
the blocked real-Wine smoke test and count each passing case once. The package
metadata description was then corrected from the old "read-only" wording;
structural checks and native packaging were rerun for that metadata-only change.

## New regression coverage

Nine Rust tests cover PE format/architecture rejection, managed-drive containment
and symlink escapes, persistent metadata identity checks, approval before I/O,
explicit process arguments with spaces, prefix/log environment, bounded output,
a real fake-runtime subprocess that creates fixture files and exits nonzero,
and missing-runtime failure reporting. Six React tests verify installer consent,
missing-runtime handling, picker cancellation, resetting approval after edits,
selection required after installer exit zero, and explicit game-launch IPC.
Frontend tests mock IPC and the file dialog. Fake-runtime tests execute a shell
fixture representing the runtime contract; they do not execute Windows code.

Initial Rust compilation found an E0308 error in runtime lookup's error type.
The return type was corrected and all 80 native cases subsequently passed.
Previous Steam diagnostics, log setup, automatic selection, repair/rollback,
and launch fixtures continue to pass.

## Build environment

Disposable Ubuntu 24.04.3 x86_64 container, no host Steam library mounted.
Workspace-local Rust/Cargo 1.99.1, Node 24.22.1, npm 11.9.0, GTK 3.24.41,
WebKitGTK 2.52.3, locked npm/Cargo dependencies. Release compilation uses one
codegen unit for the application crate, preserving the workaround documented
in the earlier build report. The package targets glibc >= 2.39 and requires
GTK/WebKit dependencies. This DEB is not a universal Linux or Steam Deck package.

Installing Wine test dependencies partially failed in the disposable container:
libibverbs group setup was denied, which left libpcap/libwine/wine64 unconfigured.
The Wine64 executable and MinGW compiler were nevertheless present. A benign
Windows console fixture was compiled with MinGW. Launching it through Wine
exited 1 due to the denied socket; it produced no installation or playability
proof. These test-only dependencies are not bundled into Proton Medic.

## Preservation and limits

The new page retains the existing charcoal/green design and other application
pages. Each approved installer gets a new persistent prefix; existing Steam
prefixes and the default Wine prefix are untouched by this flow. Profiles and
per-run logs persist, with no automatic deletion. Prefix isolation does not
restrict a Windows program's access to user files and is not a security sandbox.
Installer trust requires explicit approval. The original executable's parent
folder is the working directory so adjacent installer payloads can be found.

Users must install Wine or native Steam Proton separately. A Wine64-only runtime
may lack required 32-bit components. Flatpak-only Proton execution, runtime
downloads, MSI input, uninstall, standalone rollback, and automatic standalone
repair recipes are not implemented. The new page reads recognized signatures
from captured logs on request; existing automatic Steam monitoring remains.
Captured stdout/stderr are limited to 2 MiB each; Proton's separate own log file
is not size-capped by the writer (the diagnostic reader remains bounded).

A process exit is not installation or gameplay proof. Users must select an .exe
inside the managed Windows C: drive and test it. Closing Proton Medic loses
process tracking; on restart the profile explicitly reports unknown prior
process state. Installer/game descendants holding output pipes can keep the
run marked active until those processes close. No process is automatically
killed. Anti-cheat is not bypassed, and other runtime, driver, DRM, launcher,
or game-specific incompatibilities can still prevent play.

The complete previous v0.5.2 delivery is included byte-for-byte under backup/ in
the new ZIP, preserving the earlier source/installer/history and original v0.4.3
backup. The new ZIP includes complete source and lockfiles, tests, release notes,
this report, build evidence, and the new DEB; generated dependencies/build caches
are excluded. See PHASE-6-RELEASE-NOTES.md for the usage sequence.

## Required desktop acceptance

On a supported disposable Linux desktop, install the DEB and a suitable runtime;
run a trusted graphical installer, choose its installed game, launch it, inspect
captured logs, restart the application, and verify existing Steam features.
Repeat with native Proton, a 32-bit installer, multi-file installers, and Steam
Deck/other packaging targets before advertising broad compatibility.

## Final package installation

The final DEB installed successfully with `dpkg -i` (exit 0), upgrading the
container from 0.5.2 to 0.6.0. `dpkg-query` returned `install ok installed 0.6.0`.
The installed `/usr/bin/proton-medic` exactly matches the extracted DEB payload
(`cmp` exit 0), and `ldd` reported no missing libraries. This verifies package
installation and dynamic-library availability, not GUI startup.

Final DEB SHA-256:
`a52283c879ebbf6163d6d546615a03245281dbf701ad5c670d9541a43403ff34`

The full gate's earlier 0.6.0 package hash belongs to the pre-description-update
build. `final-package-build.log` and this final hash identify the delivered build.
Historical package filenames printed by the gate are retained build artifacts;
the delivered installer is specifically version 0.6.0.
