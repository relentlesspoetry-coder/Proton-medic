# Proton Medic v0.6.7 — header updates

## Changes

- Moved the update button to the top-right of the main app, visible on every page.
- Shows the installed app version immediately below the update button, sourced from package.json.
- Keeps one updater mounted across navigation; consent, signed updates, progress, errors, and update help remain available.
- Added a compact details panel with close/reopen controls; closing it does not cancel an active update or permit duplicate installation.
- Preserved the blue glass theme and all game, diagnostic, and repair features.
- Native startup version now comes from Cargo package metadata.

## Backup

Source before these changes is preserved on `backup/v0.6.6-before-header-update` at `dee147cf99ef8d07ebb783a134415e62ff4ab2de`. The published v0.6.6 release remains available.

## Verification

Validation is pending for this revision. No new local build, installation, desktop visual inspection, or game-launch result is claimed.

## Pending asset

The requested program logo was not attached to the request. Logo integration is pending the original image. No replacement logo was invented. This version has not been published as a release.
