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

[GitHub validation run 40772658658](https://github.com/relentlesspoetry-coder/Proton-medic/actions/runs/40772658658) passed for source commit `dcdeecdb3b0572d2eae8240c748aaa138eb84a10`:

- 92 Rust tests, 31 React tests, five Node contract/startup tests, seven disposable-game-fixture tests, five build-gate tests, and five release-manifest tests passed: 145 executable tests total.
- 22 source structure checks passed.
- TypeScript checking and the Vite production bundle passed.
- Optimized Rust/Tauri compilation and native amd64 DEB packaging passed; the CI artifact was uploaded successfully.

These checks ran on GitHub's Ubuntu 24.04 runner. No local build, installation, desktop visual inspection, live updater installation, or game-launch result is claimed for v0.6.7. This CI package has not been signed or published as a release.

## Pending asset

The requested program logo was not attached to the request. Logo integration is pending the original image. No replacement logo was invented. This version has not been published as a release.
