# Proton Medic v0.6.7

## Added

- New Proton Medic logo in the sidebar, desktop launcher icons, and repository README.
- Matching browser icon for frontend development.

## Changed

- Removed obsolete project reports, unused build configuration, and unused diagnostic-screen code.

- Moved **Check for updates** to the top-right corner on every page.
- Added the installed app version directly below the update button.
- Added a compact update-details panel that can be opened and closed.
- Kept update progress active across navigation and when the details panel is closed.
- Preserved installation consent, release notes, progress messages, error details, and signature verification.
- Read version labels from package metadata to keep them aligned with each build.

The blue glass theme, game features, diagnostics, repairs, backups, and rollback remain available.

## How it works

Click **Check for updates** to look for a newer stable release. Use the information button beside it to open update help without checking GitHub. Closing the details panel leaves an active update running and prevents duplicate installation requests.

## Validation

[Linux validation](https://github.com/relentlesspoetry-coder/Proton-medic/actions/runs/40775708785): 145 tests and 23 source checks passed. TypeScript checking, the frontend production build, native compilation, and amd64 DEB packaging passed.

Source before this update is preserved on `backup/v0.6.6-before-header-update`.
