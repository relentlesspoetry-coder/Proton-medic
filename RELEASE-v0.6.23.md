# Proton Medic v0.6.23

- Automatic Proton recommendations and fixes now use only runtimes available to the game’s Steam installation, matching the manual picker. Ambiguous library ownership blocks automatic selection.
- Steam configuration writes now use an exclusive cross-process lock and reject stale snapshots, preventing two Proton Medic instances from overwriting each other’s changes. Existing backups and rollback remain available.
- Added **Cancel queued log setup** to Game log updates. Cancel logging requests that are waiting for Steam to close without changing settings, so they no longer block closing or updating Proton Medic. Completed logging changes retain their restore receipts.
- Failed Windows installer starts now display the runtime error without an “Installer started” notice.

Existing games, prefixes, shortcuts, and configuration backups are preserved. Signed DEB and AppImage updates remain available.
