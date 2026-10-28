# Proton Medic 0.6.1 — reliability and blue glass polish

28 October 2026

## Changes

1. Interrupted Windows runs block new launches and executable changes until a
   system reboot proves the old run cannot remain alive. The profile records
   the Linux boot ID. Legacy running records acquire a boot ID on discovery;
   restart Linux after discovery to recover those records. Missing boot identity
   fails closed. No process is automatically killed. A per-profile file lock
   prevents simultaneous mutation by separate Proton Medic instances, and the
   latest persisted state is rechecked before launching.
2. Captured stdout/stderr retain the most recent 2 MiB rather than the beginning.
   Capture is bounded in memory and on disk; snapshots are written every 256 KiB
   and at EOF. During an active run, output since the last snapshot is not yet
   visible to diagnostics. Capture failures are reported in completion messages.
3. Windows-game diagnostics return per-file missing/unreadable/signature and
   partial-log states separately from findings. At-cap captured streams are
   conservatively labelled partial. Proton's separately generated log remains
   subject to the existing bounded diagnostic reader, not a writer size limit.
4. Opening a Steam game's Fix & Test panel retrieves its logging receipt,
   restoring access to the existing log-settings rollback control.
5. A failed completion save preserves its receipt in memory, keeps the job
   blocked, reports a warning and retries on library refresh. Keep the app open
   to retain that receipt; app exit cannot preserve data when disk writes fail.
   The previously persisted running state still blocks unsafe reuse after exit.
6. Vulkan output reads are nonblocking and have a six-second deadline, including
   when a descendant keeps an output pipe open. The existing child timeout is
   retained. The deadline ends output collection without killing unrelated jobs.

The blue glass theme now has its own stylesheet, imported after base component
styles. Existing cascade order was preserved to avoid unverified visual changes.
README, build instructions and design approval now point to the current release.
Application, npm lockfile, Cargo lockfile and Tauri version are 0.6.1.

## Executed verification

The final automated gate passed 85 Rust tests, 21 React tests, one Node status
contract test, seven disposable-game Python fixtures and five build-gate tests:
**119 tests**, plus **21 structural checks**. TypeScript checking, production
frontend compilation and optimized native Tauri/DEB packaging also ran in the
release gate. See verification/release.log in the source archive for exact output.

Eight added regression cases cover unknown run/reboot gating, late crash tails,
failed-save receipt retention, cross-instance locking, an open output pipe that
must return within the deadline, remounted logging restore controls, disabled
unresolved-run controls and unreadable/partial log presentation.

Initial checks found fixtures that did not create persistent profiles before
spawning, and frontend mocks that did not handle the new read-only status lookup.
Those fixtures were corrected and the full gate rerun. No real game data was used.

## Limits and acceptance

These fixes use conservative reboot-based recovery, not automatic reattachment
to Wine process trees. Logging restore receipts and failed completion receipts
remain session-scoped. No uninstall or destructive repair was added.

Real graphical Wine installers, native Proton game execution, native WebKit UI
appearance, Steam Deck and other distributions remain unverified. The current
amd64 DEB is built for Ubuntu 24.04-class systems with glibc >=2.39. This is not
universal Linux packaging. Clippy was unavailable during the review and was not
claimed as a passing check. The full native release build is the compilation
check; the process/UI tests use disposable fixtures and mocked IPC where noted.

The approved concept remains the visual target. Exact pixel equivalence has not
been verified. Generic thumbnails and real scan results replace fictional data.
Historical release reports retain their original version-specific claims.

The previous complete delivery is preserved as backup/ProtonMedic-pre-polish.zip.

## Installer verification

The delivered DEB installed with dpkg exit 0. Package status is
`install ok installed 0.6.1`. The installed binary matches the extracted payload
byte-for-byte; ldd reported no missing shared libraries. GUI startup was not
verified. SHA-256: `bc861784a6c61b1b86bcd9bce0d7ced2162267775f02ddc7bb3bbe7d824c03ba`.
