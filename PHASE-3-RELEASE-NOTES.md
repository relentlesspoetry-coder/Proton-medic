# Proton Medic v0.3.0 — repair and recovery

## Added

- **Safe Repair & Recovery** in each game's details.
- **Refresh shader cache** archives `steamapps/shadercache/<app-id>` so Steam can regenerate it.
- **Rebuild Proton environment** archives `steamapps/compatdata/<app-id>` so Steam can create a new prefix.
- A preview shows source and backup paths before a separate approval checkbox enables the action.
- A backup list supports restoring archived directories.
- Restore archives any replacement directory before moving the selected backup into place.

## How it works

The backend rediscovers the selected game from Steam manifests and checks that Steam is closed. Changes use same-filesystem renames into a private backup directory. Symlinked paths, invalid backup IDs, and cross-game restores are rejected. A process-wide lock serializes repair requests.

Backups are stored under `steamapps/.proton-medic-backups/` on the same library drive. They are retained until managed separately. If a game disappears from Steam manifests, its backup remains available for manual recovery.

## Before rebuilding a prefix

A Proton prefix can contain saves and application settings. Protect irreplaceable files separately, close Steam and the game, and check free space on the library drive.

## Validation

17 browser UI checks and TypeScript syntax checks passed. Six Rust repair tests were added; native test execution was not recorded for this version.
