# Proton Medic v0.6.13

## Installation completion

- After a successful installer exit, suggest likely game executables from the profile's Windows drive and original installer folder.
- Exclude common setup programs, uninstallers, prerequisites, and crash helpers from suggestions. Searches are bounded and do not follow directory symlinks.
- Preview the selected executable's embedded icon, review the game name, and approve saving the profile. Desktop and applications-menu shortcuts are optional.
- Keep manual executable selection available when discovery cannot find the game, including installations on other drives. Suggestions never select or launch a game automatically.

## Per-game launch settings

- Choose a runtime, supply one argument per line, and set an optional working directory under **Game settings and logs → Edit launch settings**.
- Preserve spaces within each argument and pass arguments directly to the game without shell expansion.
- Use the same saved settings for launches inside Proton Medic and through desktop shortcuts. Installer runs retain their own default arguments and working directory.
- Back up profile metadata before changes. Reset restores the original runtime, clears arguments, and uses the game's executable folder without moving the prefix or game files.
- Block changes to active or unresolved profiles, reject stale edits from other windows, and validate runtime compatibility and working-directory paths before saving or launching.

## Update

Choose **Check for updates** at the top right and approve installation. From v0.6.11 onward, Proton Medic automatically opens the installed version after updating.

Requirements: Ubuntu 24.04-compatible amd64 Linux, glibc 2.39 or newer, GTK 3, and WebKitGTK 4.1.
