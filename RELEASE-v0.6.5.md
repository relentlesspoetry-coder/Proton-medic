# Proton Medic v0.6.5 — GitHub updater

## Implemented

- Settings → App updates: manual stable-release check, available version and notes, explicit installation consent, progress, error handling, and close/reopen guidance.
- Uses Tauri's native updater, HTTPS, and the application's embedded public key. Both artifact signatures and signed version binding are required. No `git pull`, arbitrary shell download script, telemetry, or automatic restart.
- Configured endpoint: https://github.com/relentlesspoetry-coder/proton-medic/releases/latest/download/latest.json
- Signed amd64 DEB release generation. The manifest advertises only `linux-x86_64-deb`; other architectures/package formats are not falsely advertised.
- GitHub validation and release workflows. A matching stable version tag triggers checks, compilation, signing, verification, and publication. Signing credentials are scoped to the signing step.
- Source prepared as a local Git repository. Private signing material, old backup archives, build caches, screenshots, and local evidence are excluded from the staged public source.
- v0.6.4 Steam launch fix, bundled-font workaround, diagnostics, game features, and backups remain.

## Executed checks

- 92 native Rust tests passed.
- 27 React tests passed, including six new updater tests.
- Five Node contract/startup tests passed.
- Seven disposable-game-fixture tests, five build-gate tests, and five new release-manifest tests passed.
- Total: 141 executable tests, plus 22 source structure checks.
- TypeScript and Vite production build passed.
- Actual optimized Rust/Tauri application compiled and the native DEB was built.
- The exact release DEB was signed for v0.6.5 and cryptographically verified against the public key embedded in the app. Altered package bytes were rejected.
- DEB installation/upgrade in the disposable build environment passed; this is not desktop GUI verification.

A test cleanup-hook mistake and a Rust/JavaScript updater version mismatch were found during development and corrected. Final results above come from the complete successful rerun, not those failed attempts.

## Not completed or verified

GitHub returned 404 for the new repository, and the browser session was signed out. No source push or GitHub release publication is claimed. Live update checks/downloads against GitHub, GitHub-hosted workflow execution, graphical privilege prompts, actual in-app installation, rendering, and real-game gameplay have not been verified.

The update button reports a check failure until a release containing latest.json is published. The source and signing configuration are ready; finish the one-time steps in GITHUB-RELEASES.md to activate distribution.

The amd64 DEB targets compatible Ubuntu 24.04/Debian-family installations with glibc 2.39 or newer. It is not a verified universal Linux/Steam Deck package. System package updates may require administrator approval. Finish game installations and repairs before updating.

## Files

The complete source archive includes the installer and signature, updater manifest, verification logs, prepared Git bundle, and CRC-verified v0.6.4 source-and-installer backup. The separate private signing-key backup is maintainer-only and must never be published. Future releases must use the same key or a deliberate key migration.
