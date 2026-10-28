# Proton Medic v0.6.5 — signed app updates

## Added

- **Settings → App updates** with manual update checks.
- Available-version details, release notes, installation approval, and progress.
- Clear check, download, signature, and installation errors.
- Signed amd64 DEB packages and a stable GitHub update manifest.
- GitHub Actions workflows for validation, signing, verification, and publication.

## How it works

The app checks GitHub when requested and compares the latest stable version with the installed version. Tauri verifies the package signature and signed version before installation.

Finish active installations and repairs before updating. The system package manager may request administrator approval. Close and reopen Proton Medic after installation.

See [GITHUB-RELEASES.md](GITHUB-RELEASES.md) for maintainer setup and publishing.

## Validation

141 tests and 22 source checks passed. TypeScript checking, frontend compilation, native compilation, DEB packaging, package installation, signature verification, and altered-package rejection passed.
