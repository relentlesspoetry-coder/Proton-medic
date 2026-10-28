# Proton Medic v0.5.2 — automatic per-game logging

## Added

- Prepare Proton logging when diagnosing a Steam game or approving a launch test.
- Set `PROTON_LOG=1 %command%` for empty launch options.
- Preserve supported existing arguments and command wrappers.
- Back up the account's `localconfig.vdf` before changing the selected game's launch options.
- Queue logging changes until Steam closes.
- Show logging status and the backup path in Fix & Test.
- Restore the game's previous logging options while preserving unrelated changes.

The app resolves one most-recent signed-in Steam account and revalidates queued changes before writing. Ambiguous accounts, conflicting logging assignments, complex scripts, symlinked configuration, and custom log destinations are left unchanged with an explanation.

Repeated diagnosis reuses the current logging receipt. Restore saves another backup and rejects conflicting later edits. Receipts last for the app session; backups remain on disk.

Logging setup does not launch the game. A new Proton log is produced on the next Proton launch.

See [TEST-REPORT-v0.5.2.md](TEST-REPORT-v0.5.2.md) for test results.

Reference: [Valve Proton FAQ](https://github.com/ValveSoftware/Proton/wiki/Proton-FAQ).
