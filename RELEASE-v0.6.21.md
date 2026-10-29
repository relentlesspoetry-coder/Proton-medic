# Proton Medic v0.6.21

- Simplified the Proton versions page and moved setup instructions into expandable Help.
- Steam games now list only Proton versions available to their Steam installation. Duplicate version names include their paths.
- Changing the download destination clears the previous approval, including when a refresh detects a different Steam installation.
- Inventory refreshes every 30 seconds while idle and every second during downloads. Refreshing pauses while the window is hidden and resumes when it becomes visible.
- Added distinct loading, empty, and unavailable inventory states, with a retry action for failed reads.

Existing Proton versions, game settings, and configuration backups are preserved. Signed DEB and AppImage updates remain available.
