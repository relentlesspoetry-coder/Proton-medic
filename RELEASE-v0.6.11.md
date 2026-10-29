# Proton Medic v0.6.11

## Updates

- Automatically reopen Proton Medic after an update installs successfully, so the installed changes take effect immediately.
- Verify the installed executable reports the expected version before relaunching. Relaunch uses the saved installation path after the package replaces the old executable.
- Restore the original font environment for the new process, preserving bundled-font initialization.
- Apply updates without a separate restart button or another confirmation after installation. Report a startup error if the installed program cannot be opened.
- Stop update preparation while Windows games, installers, completion receipts, queued logging setup, or Fix & Test sessions need attention. New launches and configuration changes wait while an update is in progress.
- Release the update guard when downloading or installation fails, allowing normal work and a later retry.

## Install

Use **Check for updates** at the top right. Review the release notes and approve installation and automatic relaunch.

When installing this release from v0.6.10 or earlier, close and reopen Proton Medic once to load the new updater. Subsequent updates reopen automatically.

Requirements: Ubuntu 24.04-compatible amd64 Linux, glibc 2.39 or newer, GTK 3, and WebKitGTK 4.1.
