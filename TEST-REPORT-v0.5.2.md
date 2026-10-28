# Proton Medic 0.5.2 — automatic log setup verification

Date: 28 October 2026

## Outcome

Automatic log setup is implemented for the targeted supported Steam configuration
paths. Actual Rust/Tauri test compilation and optimized release compilation
succeeded. The DEB was built and installed in the disposable Ubuntu environment.
Real Steam-client behavior and generation of a real game's log remain unverified.

## Executed checks

| Check | Result |
| --- | --- |
| Native Rust unit/fixture tests | 71 passed; 0 failed/ignored |
| Python disposable launcher fixtures | 7 passed |
| Python build-gate tests | 5 passed |
| Frontend/native status contract | 1 passed |
| Final React/Vitest/jsdom suite | 12 passed (8 Fix & Test + 4 monitor) |
| Syntax/config/structural checks | 20 passed |
| Strict TypeScript and Vite production build | Passed |
| Full native release gate | Exit 0 |
| Optimized Tauri application and DEB build | Passed |
| DEB inspection and `dpkg -i` | Passed: install ok installed 0.5.2 |
| Installed executable vs extracted package payload | Exact byte match |
| Shared-library resolution (`ldd`) | No missing libraries |
| Native GUI / real Steam / real game log generation | Not executed |
| SteamOS and cross-distro validation | Not executed |

Total: **96 executable tests plus 20 structural checks**. The full gate included
11 React cases; a final additional waiting/setup regression brought that suite to
12, and the complete frontend suite was rerun successfully afterward. That last
addition was test-only, so it did not require another application rebuild.

## New coverage

Eleven native tests validate empty/argument-only options; existing wrappers and
logging assignments; unsafe/ambiguous script rejection; minimal target-leaf edits;
creation of missing game/apps records only in a recognized layout; duplicate-key
rejection; real backup/enable/restore file operations; idempotency; conflict refusal;
restoration of an originally absent field; most-recent-account resolution;
account ambiguity; symlink refusal; and consent before mutation.

Three frontend cases were added to the previous suite: setup is requested before
diagnosis and a setup error stops that flow; the original log settings can be
restored through the native command; and queued setup does not consume a launch
attempt. Existing tests were updated to account for the setup step.

The Rust file-operation tests use throwaway localconfig/userdata fixtures, not a
live Steam profile. Frontend tests mock IPC in jsdom, not native WebKit. The
queued Steam-close lifecycle and live-client handling of edited launch options
still require end-to-end target-machine validation. Prior native fixture tests
for scanner, actual fake-launcher subprocesses, diagnostics, repair and rollback
continue to pass.

## Preservation and limits

Existing launch arguments are preserved for supported forms, and localconfig is
backed up before editing. The selected account is identified from one MostRecent
record and revalidated after a queued wait. Unknown/ambiguous layouts, complex
shell expressions, custom log destinations and symlinked user configuration are
refused. Such cases can block automatic setup or guided launch pending review.
No shell command is executed by setup. Games are not launched simply by diagnosis.

Logging may already be enabled, in which case no write/backup is needed. The
setup receipt is retained on repeated diagnosis. Restoring affects only the
selected game's options and refuses a newer value; it also backs up the displaced
configuration. Receipts do not persist across app exit; backup files do.

The existing game log reader watches standard native/Flatpak Proton locations.
This version does not add standalone-game log formats or runtime installers.
A real log requires an actual Proton launch, and successful configuration alone
is not proof that a log was generated or that gameplay works.

The stylesheet is unchanged. Existing discovery, diagnostics, repair, automatic
selection and launch features remain. Automatic setup is now an additional
prerequisite for the supported guided diagnosis/launch flow.

## Environment and artifact

Ubuntu 24.04.3 LTS x86_64 disposable container; Rust/Cargo 1.99.1; Node.js 24.22.1;
npm 11.9.0; Tauri 2.12.5; GTK 3.24.41; WebKitGTK 2.52.3. Cargo/npm lockfiles are
included. The application retains the one-unit release codegen configuration.

The installer requires glibc >=2.39 and is not a universal Linux or SteamOS
package. Display/socket restrictions from the earlier environment still prevent
claiming a verified native GUI startup. No remote CI execution is claimed.

DEB SHA-256:
`c8cddc7ea6ef5a58ebe150ff49d9581d789dae9efb7a9b43dd7745e5fc919bbb`

Unmodified v0.5.1 delivery backup SHA-256:
`30a37f1a5e2a35e0948285320564d5ccd194449db0d7872474810dcf7b45944f`

## Evidence

`phase52-evidence/release-gate.log`, `rust-tests.log`,
`verified-frontend-tests.log`, `frontend-build.log`, `package-info.log`,
`install.log`, `package-verified.log`, and `ldd.log`. An earlier redirected
frontend log was incomplete; the final tool-output transcript records the full
12-case interaction suite with exit 0. Older cached DEBs appear in the gate;
the v0.5.2 artifact was independently installed and checked.

Reference: [Valve Proton FAQ](https://github.com/ValveSoftware/Proton/wiki/Proton-FAQ)
documents `PROTON_LOG=1 %command%`. This supports the logging setting, not a claim
that this release's Steam configuration integration was tested against a live
client.
