# Proton Medic v0.6.49

- Repair existing game shortcuts after moving the AppImage or downloading a newer version to a different filename. Open the new application and use Create desktop shortcut for the game. Previous launcher files are backed up before replacement.
- Keep unrelated or customized launchers, different game profiles, ambiguous commands, and symbolic links unchanged.
- Fix desktop launchers for a native application executable whose path contains a percent character.
- Add regression coverage for moved applications, legacy shortcuts, literal special characters, backup preservation, and refusal cases.

DEB: Ubuntu 24.04-compatible amd64 Linux. AppImage: x86_64 Linux with glibc 2.35 or newer and working graphics drivers. Steam Deck hardware-specific rendering and controller behavior are not newly validated by this release.
