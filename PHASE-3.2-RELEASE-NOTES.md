# Phase 3.2 — Automatic Proton Selection (candidate build)

This update implements per-game compatibility-tool selection, not merely a recommendation.

- UI: opt-in automatic selection in Settings, one-click selection in game details, real status feedback and safety disclosure; clean approved dashboard retained.
- Advisor: installed Proton versions ranked from Steam configuration and log evidence, skipping missing installations and clear system-level errors.
- Native: validated scanned App ID / library, freshness gate (48 hours), exact tool ID from custom manifest where available, Steam-running blocker, minimal VDF modification, config backup, atomic write, preservation of other mapping entries, and no arbitrary path inputs.
- Native tests added for inserting and changing mappings, preserving comments and defaults, avoiding unnecessary rewrites, rejecting unsafe IDs, handling fresh Steam config, custom GE identifiers, and atomic backup/write.
- Browser smoke tests: 22 passed, 0 failed, mocked backend.
- Native compilation, real Steam integration, and filesystem tests: **not run** due to unavailable Rust/Cargo.

The app **cannot determine the uniquely correct Proton version from a failure log alone**. For game-specific proof it must launch and observe success (planned). This release chooses a plausible installed candidate and applies it safely after the user enables the feature. Automatic game launch / switching between versions is not included.
