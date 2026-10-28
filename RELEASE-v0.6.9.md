# Proton Medic v0.6.9

## Added

- Rename and Remove from list buttons on each Windows game card.
- A confirmation step before removing a game and a Removed games section for restoring it.
- Windows game icons extracted from the selected executable and cached as PNG images.
- Steam game icons loaded from the local Steam artwork cache.
- Game icons in newly created or refreshed desktop shortcuts.

## Changed

- Moved the name editor out of the diagnostic details so it is easy to find.
- Kept icon loading separate from library polling to avoid rereading executable resources on every refresh.
- Reused the Steam scan index for artwork lookups instead of rescanning libraries for each game image.
- Preserved backups before changing profile names, list membership, or an existing Proton Medic shortcut.

## How it works

Choose Rename on a Windows game card, enter a display name, and save it. Choose Remove from list and confirm to hide a profile. Game files, prefixes, saves, and existing desktop shortcuts remain in place. Expand Removed games and choose Restore to show a hidden profile again. Running games and installers cannot be renamed or removed.

Select a game executable to load its embedded icon. Steam artwork uses files already cached on this computer. Missing or unreadable artwork displays the standard game symbol. Icon extraction reads executable resources without launching the file or contacting an online service.

Choose Create desktop shortcut again to refresh an existing Proton Medic launcher with its current name and icon. The previous launcher is backed up in the profile directory. Unrelated shortcuts are left unchanged.
