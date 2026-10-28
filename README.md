# Proton Medic

<p align="center"><img src="src/assets/proton-medic-logo.webp" alt="Proton Medic" width="420" /></p>

Proton Medic helps Linux players diagnose game problems, manage installed Proton versions, and apply repairs with backups and rollback.

**Version:** v0.6.8 · [Downloads](https://github.com/relentlesspoetry-coder/Proton-medic/releases/latest)

## Features

- Find native and Flatpak Steam installations, game libraries, and installed Proton tools.
- Analyze Proton logs and check Vulkan and system information.
- Recommend installed compatibility tools based on game settings and log findings.
- Launch selected Steam games and inspect new logs.
- Configure per-game Proton logging and monitor logs for changes.
- Apply supported configuration fixes with backups and selective rollback.
- Run trusted Windows `.exe` installers using an installed Wine or Proton runtime.
- Register installed Windows games using their existing prefix, including game files on other drives.
- Create desktop and applications-menu shortcuts that reuse a game’s saved environment.
- Check for signed GitHub updates when requested.
- Use a blue glass interface with bundled fonts.

The update button appears at the top right on every page, with the installed version directly below it.

## Install

Download the amd64 DEB from [Releases](https://github.com/relentlesspoetry-coder/Proton-medic/releases), then install it with your package manager. Existing installations can check for updates in the app.

**Requirements:** x86_64 Linux compatible with Ubuntu 24.04, glibc 2.39 or newer, GTK 3, and WebKitGTK 4.1. The DEB declares its package dependencies.

## Windows game launchers

Open **Windows Games**, select the installed game’s `.exe` in its card, then choose **Create desktop shortcut**. Shortcuts launch through Proton Medic’s native launcher using the saved runtime, prefix, and working directory. The main window does not need to be open. GNOME may ask you to **Allow Launching** on first use.

Use **Add installed game** for an installation missing from the list. Select its executable and the existing prefix containing `drive_c` and `system.reg`; use the runtime that previously ran it. This registers the game without reinstalling or copying it. Games installed through Proton Medic may initially appear under their installer’s name. Use **Game settings and logs** to rename them.

## Build

Install Rust 1.99.1, Node.js 22 or newer, and the dependencies listed in [BUILDING.md](BUILDING.md).

```bash
npm ci
bash scripts/build-zorin.sh --check
bash scripts/build-zorin.sh --build
```

Use `npm run dev` for the frontend or `npm run tauri -- dev` for the desktop app.

## How repairs and updates work

Scans and log analysis stay on your computer. Configuration changes preserve backups and require Steam to be closed. Game launches and manual repairs require approval. Automatic fixes are optional and limited to supported log findings.

Update checks contact GitHub only when requested. Packages are signature-verified before installation. System package installation may request administrator approval.

Proton Medic does not bypass anti-cheat or DRM. Launch status and log findings help diagnose a game; the player confirms whether it runs correctly. Windows installers run with your user permissions, so use trusted files.

## Documentation

- [Build instructions](BUILDING.md)
- [Signed releases and updater setup](GITHUB-RELEASES.md)
- [v0.6.6 changes](RELEASE-v0.6.6.md)
- [v0.6.7 changes](RELEASE-v0.6.7.md)
- [v0.6.8 changes](RELEASE-v0.6.8.md)

