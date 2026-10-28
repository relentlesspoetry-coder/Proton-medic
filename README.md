# Proton Medic

A Linux desktop application for investigating gaming problems, inspecting Proton logs, selecting installed compatibility tools, and trying backed-up, supported repairs.

Current development release: **0.6.6**. Built with Rust, Tauri 2, React, and TypeScript.

## Features

- Native and Flatpak Steam discovery, game libraries, and installed Proton tools.
- Log analysis, Vulkan/system diagnostics, and evidence-based Proton recommendations.
- Approved game launch checks that work independently of automatic log setup.
- Per-game logging setup when Steam configuration can be safely edited.
- Optional monitoring of new logs and supported backed-up configuration fixes.
- Repair backups and selective rollback; no destructive automatic repair.
- Trusted Windows `.exe` installer support using an existing Wine or Proton runtime.
- Blue glass interface with bundled fonts to avoid problematic system-font enumeration.
- A Settings update button for signed GitHub release packages.

A successful launcher command or clean log does not prove gameplay works. Unsupported anti-cheat, DRM, missing system components, and game-specific failures may require publisher support or manual investigation. Proton Medic does not bypass anti-cheat or guarantee every Windows game will work.

## Linux support

The native amd64 DEB is built on Ubuntu 24.04 and requires glibc 2.39 or newer plus the package's GTK/WebKit dependencies. Zorin OS 18 is the current user test target. Universal distro support and Steam Deck packaging are goals, not verified guarantees. Windows installers execute user-selected programs; Wine prefixes are not security sandboxes.

## Development

Install Rust 1.99.1 and Node.js 22 or newer, plus the native packages listed by:

```bash
bash scripts/build-zorin.sh --help
npm ci
bash scripts/build-zorin.sh --check
bash scripts/build-zorin.sh --build
```

The build gate runs tests against disposable fixtures and builds the actual Rust/Tauri application. It does not launch real Steam games or change a real Steam library.

For frontend development use `npm run dev`; for the desktop app use `npm run tauri -- dev`.

## Releases and verification

See [GITHUB-RELEASES.md](GITHUB-RELEASES.md) for signed updates and CI setup, and [RELEASE-v0.6.6.md](RELEASE-v0.6.6.md) for the current verification report. Earlier release reports in the source are historical.

Private signing keys are not included in this repository. Keep the separately supplied maintainer key backup secure.

## Privacy and safety

Game scans and log analysis stay local. Update checks contact GitHub only when requested. Game repair actions preserve backups and require the applicable approval and Steam-closed checks. System package installation may request administrator approval.
