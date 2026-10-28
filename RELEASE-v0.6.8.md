# Proton Medic v0.6.8

## Changed

- Simplified game details by removing repeated recommendations.
- Kept automatic-fix controls in Settings, with compact log alerts on other pages.
- Consolidated the blue glass styles and improved wrapping for headers, buttons, game names, paths, and installer fields.
- Moved the managed Windows games list above the installation form.

## Added

- Desktop and applications-menu shortcuts for registered Windows games. Shortcuts use the saved runtime, prefix, and working directory without opening the main window.
- Registration of existing Windows games with their original prefix, without copying or reinstalling game files.
- A bounded executable search in a profile’s Windows C: drive and original installer folder.
- Editable game display names under Game settings and logs.

## Fixed

- Allowed explicitly selected game executables on other drives to use their existing managed prefix.
- Kept the profile locked while a game or installer is running, including launches from desktop shortcuts.
- Recognized games launched by another Proton Medic process as running.

## Usage

In Windows Games, select the installed game executable in its card, then choose Create desktop shortcut. If GNOME asks, choose Allow Launching. Keep Proton Medic installed so its native launcher remains available.

If a game is missing, use Add installed game, choose its executable and the prefix containing drive_c and system.reg, and select the runtime used for that installation. Installer profiles can appear under the setup executable’s name until renamed.
