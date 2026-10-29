# Proton Medic v0.6.25

- Windows game supervisors retain profile, prefix, and update locks if Proton Medic closes unexpectedly. Launch results and logs are saved when the game finishes, allowing the profile to recover without rebooting.
- Completed repairs stay completed in the interface. Delayed session responses cannot bring back obsolete Keep or Rollback controls or hide Diagnose.
- Disabling automatic fixes invalidates pending work before configuration changes. The interface confirms read-only monitoring after any configuration write already in progress finishes.

Existing game files, profiles, and backups are preserved. Signed DEB and AppImage updates use the existing updater key.
