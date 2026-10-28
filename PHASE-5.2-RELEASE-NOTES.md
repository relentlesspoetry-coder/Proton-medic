# Proton Medic 0.5.2 — automatic per-game log setup

Choosing **Diagnose this game**, or approving a game launch test, now prepares
Proton logging for the selected Steam game. Users do not need to type a launch
option in supported configurations.

- Locate the selected game's Steam installation and its uniquely most-recent
  signed-in account. Resolve the account's `userdata/<account>/config/localconfig.vdf`.
- Add `PROTON_LOG=1 %command%` for empty launch options; preserve argument-only
  options after the command and preserve supported existing command wrappers.
- Update an existing unambiguous `PROTON_LOG` assignment to `1` without changing
  other arguments. Duplicate/conflicting assignments are rejected.
- Back up localconfig before a minimal atomic edit to this game's LaunchOptions.
  Other games, unrelated fields, comments, and formatting outside the edited field
  remain intact. Existing logs are not deleted.
- If Steam is open and a write is needed, queue setup. The automatic monitor
  processes the authorized request after Steam closes, even when automatic
  gameplay repairs are disabled. The app must remain open.
- Show the setup state and backup path in Fix & Test. **Restore previous log
  settings** restores only this game's prior launch options, preserving unrelated
  later edits. It refuses a changed target value and saves another backup.
- Repeated diagnosis does not create redundant backups or discard an existing
  same-session restore receipt. Waiting for setup does not consume the guided
  launch-test attempt.

Setup itself does not launch the game. A new log is generated when the game is
subsequently launched through Proton; launching still requires user approval.

## Supported scope and exceptions

The current implementation targets Steam-discovered games with a recognized
localconfig layout and one most-recent account. It does not guess among accounts
or silently rewrite arbitrary launch scripts. Ambiguous accounts/fields, symbolic
links in the user-config path, complex shell expressions, escaped scripts and
custom PROTON_LOG_DIR destinations are left unchanged with an explanation.
Those cases can block automatic setup and the guided launch until reviewed.
Native Linux executables do not produce Proton logs merely because this variable
is present. Standalone Windows-game integration remains future work.

Queued account changes are revalidated before writing. Logging restore receipts
are session-local; backups persist across app exit. When a missing apps/game
object is created, restoring removes the added LaunchOptions field; harmless
empty objects may remain. No user data is deleted.

The existing automatic reader, supported fix recipe, Steam discovery, diagnostics,
manual Proton selection, prefix/cache backup, rollback and charcoal/green styles
remain. The old manual log-entry instruction is replaced with automatic setup
instructions. The complete v0.5.1 delivery is backed up in the archive.

Implementation reference: [Valve Proton FAQ — enabling logs](https://github.com/ValveSoftware/Proton/wiki/Proton-FAQ).
The choice of per-account Steam configuration layout is validated with fixtures;
real Steam-client interoperability still needs target-machine testing.
