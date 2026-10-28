# Proton Medic 0.5.0 — Phase 5A: Fix & Test

This milestone adds the first coordinated diagnosis → backed-up change → launch
observation → keep/rollback workflow. It is the first repair recipe, not a claim
that all Linux gaming failures can now be fixed.

## User flow

Open My Games, select a Steam game, and choose **Diagnose this game** in the new
Fix & Test section. If a supported fresh finding and an installed alternative
Proton version are available, review the candidate, close Steam, and approve
the change and game launch. **Fix & Test** backs up the Steam configuration,
changes the selected game's Proton mapping, then observes one 10-second launch.
The game may continue running afterward. Keep the selection after evaluating
playability, or close Steam and choose **Roll back fix**.

No loop automatically cycles Proton versions. No launcher exit code or clean
log is described as proof that the game is playable.

## Implemented

- Fresh-log eligibility and revalidation immediately before applying the fix.
- One supported recipe: try a different installed Proton for Wine exception or
  Vulkan device-loss evidence, using the existing advisor and native launcher.
- Explicit approval for configuration changes and launching a game.
- Same-session rollback of only the selected game's compatibility mapping.
  Unrelated Steam edits are preserved. A changed/missing target mapping,
  ambiguous mapping, invalid config, or symlink is refused instead of overwritten.
- Backups of both the original configuration and the configuration displaced by
  rollback, using the existing atomic write/backup primitive.
- An attempt limit: one launch observation per fix session, including failed
  attempts. The user retains access to the backup and rollback after failure.
- UI recovery when navigating away from and back to game details, through
  sessionStorage plus the backend's in-memory session receipt.
- A narrowly matched unsupported-anti-cheat finding, distinct from an ordinary
  anti-cheat mention. Both the workflow and advisor refuse a Proton change for
  that explicit blocker. This is not comprehensive anti-cheat detection.
- Existing charcoal/green styling retained without stylesheet changes; all old
  discovery, diagnostics, advisor, auto-selection, launch, backup, and repair
  controls remain. Sidebar version now reads 0.5.0.

## Deliberate limits

Keep Proton Medic open until keeping or rolling back the fix. In-memory rollback
receipts do not survive app exit. Full configuration backups remain on disk;
after exit, automated per-game rollback recovery is not yet implemented. No
backup is automatically deleted. Session capacity is bounded to 32.

Only Steam-discovered games and already-installed Proton alternatives are
supported by this new recipe. System package repair, DLL/runtime installation,
standalone Windows installer import, native Linux repair recipes, store account
integration, and runtime downloads are not implemented in this milestone.

The package remains an Ubuntu 24.04-class amd64 DEB (glibc >=2.39). Broad-distro
Flatpak/AppImage distribution and SteamOS validation are still pending.

## Following milestones

1. Persist validated per-game rollback receipts across restarts and provide a
   repair-history view before expanding unattended repairs.
2. Add isolated standalone-game environments and managed Wine/Proton runtimes;
   execute imported Windows installers and collect their logs.
3. Expand the versioned repair catalog with narrowly scoped, reversible recipes
   for proven dependency and graphics failures. Never execute arbitrary log text.
4. Add Flatpak/AppImage delivery and test Ubuntu/Zorin, Debian, Fedora, Arch,
   and SteamOS on real desktop/session and filesystem configurations.
5. Add store integrations and unified install/update management.

Anti-cheat bypass is outside scope. Unimplemented repairs and other hard blockers
must be reported accurately, not marked fixed.
