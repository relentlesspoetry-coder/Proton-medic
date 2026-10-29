# Proton Medic v0.6.24

- Windows game supervision now retains detached launcher children until they finish. Prefix locks, update protection, launch history, and resource monitoring remain active throughout the run.
- Prefix archive and restore operations now share cross-process locks with Windows game launches. Locks remain valid while a compatibility directory is moved.
- Native and Flatpak Steam installations retain their own configured Proton selections. Shared libraries with ambiguous ownership do not supply automatic repair settings or log evidence.
- Diagnostics, launch tests, and automatic monitoring now scope Proton logs to the selected Steam installation.
- Unfinished Fix & Test sessions can be recovered from the backend. Keep or roll back a selection from any page, including when browser storage is unavailable or the game drive is disconnected.
- AppImage game shortcuts now preserve no-FUSE launching. Refreshing an existing shortcut backs up its previous contents.

Existing game files, profiles, and configuration backups are preserved. The DEB requires Ubuntu 24.04-compatible amd64 Linux. The AppImage requires x86_64 Linux with glibc 2.35 or newer and working graphics drivers.
