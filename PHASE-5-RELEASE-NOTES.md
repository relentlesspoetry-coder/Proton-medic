# Proton Medic v0.5.0 — Fix & Test

## Added

- A guided diagnosis, configuration change, launch check, and rollback workflow.
- An installed-Proton alternative for supported Wine exception and Vulkan device-loss findings.
- Approval before changing settings or launching a game.
- Selective rollback of the selected game's compatibility mapping.
- Backups of both the original configuration and any configuration displaced during rollback.
- Recovery of the active fix panel when navigating away and back.
- Separate handling of explicit unsupported anti-cheat findings.

## Use

Open **My Games**, select a game, and choose **Diagnose this game**. Review the recommended Proton alternative, close Steam, and approve **Fix & Test**.

The app backs up Steam configuration, changes that game's mapping, and observes one ten-second launch. After checking the game, keep the selection or close Steam and choose **Roll back fix**.

## Behavior

Each fix session allows one launch attempt, including failed attempts. Rollback preserves unrelated settings and rejects conflicting target changes. Up to 32 sessions can be tracked.

Keep Proton Medic open until keeping or rolling back a fix. Session receipts are held in memory; configuration backups remain on disk after exit.

See [TEST-REPORT-v0.5.0.md](TEST-REPORT-v0.5.0.md) for test results.
