# Proton Medic v0.6.0 — Windows installer support

## Added

- A Windows Games page with a native `.exe` picker.
- Selection of an installed Wine or native Steam Proton runtime.
- A separate persistent Wine/Proton prefix for each installation.
- Installer approval, executable selection, game launching, and runtime log analysis.
- Captured stdout and stderr, limited to 2 MiB each.

## Use

1. Open **Windows Games** and select a trusted Windows `.exe` installer.
2. Enter a game name and select an installed runtime.
3. Approve and run the installer. Complete its normal installation steps.
4. Select the installed game's `.exe` inside the profile's Windows C: drive.
5. Launch the game and inspect its logs if needed.

## Behavior

Installer and runtime paths are passed as separate process arguments. The executable's parent folder is the working directory, keeping adjacent installer data accessible.

The app checks for x86/x64 Windows PE files. This is a format check; use trusted installers because Wine prefixes do not restrict access to your user files.

Profiles and logs persist across restarts. Running processes are limited to four, with one per profile. This version reports interrupted runs as having unknown process status after restart.

Runtime installation, MSI input, ARM Windows binaries, Flatpak-only Proton execution, uninstall, and standalone rollback are outside this version.

See [TEST-REPORT-v0.6.0.md](TEST-REPORT-v0.6.0.md) for test results.
