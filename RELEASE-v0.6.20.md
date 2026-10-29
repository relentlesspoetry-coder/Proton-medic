# Proton Medic v0.6.20

- Added a Proton versions page for downloading current and older GE-Proton releases from the maintainer's official GitHub repository.
- Choose native or Flatpak Steam as the installation destination. Downloads show progress, support cancellation, and are verified against the published SHA-512 checksum before extraction.
- Versions install side by side. Existing installations are never replaced, and installing a version does not change game settings.
- Added a manual Proton selector to each Steam game's Proton tab. Close Steam before saving; the previous configuration is backed up.
- Downloaded native GE-Proton versions use an installed UMU launcher for Windows games, preserving their existing prefix and launch settings. These versions become available in the runtime selector when `umu-run` is on PATH.
- Corrected recognition of compatibility-tool IDs in nested Steam manifests.

Open **Proton versions → Load latest releases**, select a version and destination, approve the download, and keep Proton Medic open until installation finishes. Restart Steam afterward. Select versions under **My Games → game → Proton**, or **Game settings and logs → Edit launch settings** for Windows games with UMU installed.

Valve Proton and Proton Experimental remain managed through Steam's Library → Tools. Flatpak Steam downloads are available to Flatpak Steam only. Signed DEB and AppImage updates remain available.
