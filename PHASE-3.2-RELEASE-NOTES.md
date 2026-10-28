# Proton Medic v0.3.2 — automatic Proton selection

## Added

- An opt-in automatic-selection setting and a manual selection action in game details.
- Ranking of installed Proton versions using Steam configuration and recent log findings.
- Per-game compatibility mapping updates with configuration backups and atomic writes.
- Steam-running checks, App ID validation, a 48-hour evidence limit, and support for custom GE tool identifiers.
- Tests for mapping insertion, replacement, comment preservation, unchanged settings, invalid IDs, and backup creation.

Selection changes only the selected game's mapping and preserves unrelated entries. System-level errors are shown for review instead of prompting a Proton switch. This version selects an installed candidate; it does not cycle through versions or launch games automatically.

## Validation

22 browser smoke tests passed with a simulated backend. Native test execution was not recorded for this version.
