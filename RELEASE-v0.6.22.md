# Proton Medic v0.6.22

- Prevented simultaneous launches from different game profiles that share one Windows prefix, including desktop shortcuts.
- Added update coordination across Proton Medic windows and shortcut processes. Active games, installers, downloads, and repair sessions block app updates; an active update blocks new work.
- Added a window-close guard to preserve supervision while games, installers, downloads, or repair work are active. Closing remains available after a failed automatic reopen of an installed update.
- Windows Games now clears approval if a refresh replaces the selected runtime.
- Runtime-log findings are cleared when a new run starts, and delayed responses from previous runs are ignored.

Existing games, prefixes, shortcuts, and configuration backups are preserved. Shared update coordination applies to Proton Medic processes using the same application data directory. Signed DEB and AppImage updates remain available.
