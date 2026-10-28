# Proton Medic v0.6.4 — Steam launching and layout fixes

## Fixed

- Allowed Steam launch requests to continue when automatic logging cannot resolve an account.
- Skipped account lookup and configuration edits when Steam is running.
- Turned logging-setup errors and queued setup into warnings instead of launch blockers.
- Kept launcher failures separate from logging warnings in Launch & Check and Fix & Test.

When Steam is closed, the app attempts logging setup before launch. Ambiguous account settings remain unchanged. Configuration repairs still require Steam to be closed.

## Changed

- Corrected featured-card icon and text colors.
- Improved the Steam status icon, metric spacing, and sidebar sizing.
- Arranged metrics in two columns at narrower desktop widths.

The launcher uses native or Flatpak Steam to open the selected App ID, then checks for fresh logs for ten seconds. The player confirms whether the game runs correctly.

## Validation

130 tests and 21 source checks passed. TypeScript checking, the frontend production build, native compilation, and DEB packaging passed.
