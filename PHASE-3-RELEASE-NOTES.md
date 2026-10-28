# Proton Medic v0.3.0 — Phase 3: opt-in repair and recovery

## Implemented

- Preserved the v0.2.1 approved clean UI, scanner, and diagnostic pages.
- Added **Safe Repair & Recovery** on each game's details screen.
- **Refresh shader cache:** move `steamapps/shadercache/<app-id>` into a hidden backup directory within the *same Steam library*; allow Steam to regenerate it.
- **Rebuild Proton environment (advanced):** move `steamapps/compatdata/<app-id>` into that backup directory; Steam can rebuild it on launch. This may contain local save files and application settings. It is never suggested as a guaranteed fix.
- Preview source/backup directories and show risk before enabling any action; a separate explicit checkbox is required.
- Backend independently re-discovers the selected game from the installed Steam manifest. It never accepts an arbitrary frontend-supplied target path.
- Block changes while Steam or Steam Web Helper processes appear to be running.
- Refuse symbolic links at managed source/parent/vault paths; reject malformed and cross-game backup IDs.
- Use same-filesystem directory rename for backups, never deletion or a copy/delete sequence. The backup directory is created private (0700 on Unix).
- List and restore archived copies. If Steam regenerated the directory, restoration first archives that new directory rather than overwriting it. Errors never intentionally delete backups.
- Serialize mutation requests with a process-wide lock.
- Retain per-game diagnostic evidence and the UI's separate simple/advanced presentation.

## Important safety limitations

- A Proton prefix can contain saved games and unsynced user data. Do not reset it unless you understand this risk and have separately protected irreplaceable data.
- Stop Steam and the game completely, including background processes. Proton Medic will refuse to proceed if Steam is detected, but process detection is not a guarantee against races/relaunch.
- Backups consume disk space **on the same drive**. Nothing is permanently deleted or auto-pruned.
- A backup is only accessible from the in-app Restore list while its game is still discoverable in Steam manifests. If the game disappears, the data remains under `steamapps/.proton-medic-backups/` and can be recovered manually after inspection.
- Backup paths are not synced between devices. Permission errors, mount changes, Steam updates, and concurrent external changes can interrupt operations.
- These operations are not fixes for anti-cheat restrictions, all Vulkan issues, broken drivers, or other unrelated errors.
- No arbitrary commands, root access, remote uploads, or telemetry.

## Verification

- 17 / 17 Phase 3 and regression browser UI smoke checks pass in a headless Chromium sandbox with simulated Tauri backend responses.
- TypeScript TSX syntax transpilation checked using the installed TypeScript compiler.
- 6 new Rust unit tests cover name validation, preservation, restore with collision, and symlink protections. **Not executed**: Rust/Cargo is not installed in the current sandbox.
- **Native Tauri packaging, Rust compilation and live Steam filesystem integration remain unverified** here. Perform validation on Zorin OS before trusting the write features on real games.
