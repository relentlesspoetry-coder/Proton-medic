# Proton Medic v0.6.4 — Steam launch and UI update

The v0.6.3 launcher stopped before invoking Steam when automatic Proton logging could not identify exactly one most-recent account. The screenshot reported this exact error. It did not establish a game or Proton failure.

## Changes

- A running or possibly running Steam client bypasses account resolution and log-setting edits on launch. The launcher sends the selected app ID through the installed client's command (`steam -applaunch`, or `flatpak run com.valvesoftware.Steam -applaunch` for a Flatpak library).
- When Steam is verified closed, automatic logging setup is attempted. A logging setup failure or queued status becomes a visible warning, not a launch blocker. Ambiguous account settings remain untouched.
- Both Launch & Check and Fix & Test use this behavior. Diagnosis can still inspect available evidence when automatic logging is unavailable. Unsupported repairs remain unavailable.
- The launch observation checks fresh logs for ten seconds. Neither a launcher exit code nor an absence of log errors proves gameplay works.
- The blue glass theme corrects featured-card icon/text colors, the Steam status icon, cramped metrics, and sidebar sizing. At narrower desktop widths, metrics use two columns. Semantic warning/error colors remain.
- The bundled-font blank-window fix is unchanged. Existing backups, rollback, game discovery, repair guards, and Windows installer support remain.

## Executed verification

- 92 Rust tests passed, including three new launch regressions: running Steam skips account setup; an account error still executes a disposable fake launcher; queued logging preserves launcher failures and warnings.
- 21 React tests passed, including revised tests for diagnosis and approved launching with ambiguous accounts and queued logging.
- Five Node contract/startup tests passed; 21 source structure checks passed.
- Seven disposable-game fixture tests and five build-gate tests passed.
- Total: 130 executable tests passed, plus 21 structural checks.
- TypeScript checking and Vite production build passed.
- Optimized Rust/Tauri 2 native compilation and DEB packaging passed with locked Rust dependencies.

## Limits

Tests use disposable game fixtures and fake Steam launchers. Real Steam forwarding, games, Proton gameplay, and the updated UI rendering were not verified in this container. No real game compatibility claim is made. Please verify launching with Steam already open on the target PC.

Automatic logging cannot be newly enabled while Steam owns its configuration, or while the account is ambiguous; the launcher uses existing settings and reports this limitation. Log-based diagnosis may have no new evidence until logs are configured. Configuration repairs still require Steam closed.

This amd64 DEB was built on Ubuntu 24.04 for compatible Debian/Ubuntu/Zorin systems. It is not a verified universal Linux installer or Steam Deck package.

## Previous version and source

The complete archive contains the updated source, installer, final release log, and a CRC-verified v0.6.3 source-and-installer backup. Development dependencies and build caches are excluded. Historical release reports remain in the source for context; this report describes v0.6.4.
