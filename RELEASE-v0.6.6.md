# Proton Medic v0.6.6 — UI cleanup

## Changes

- Consolidated the blue glass theme into one organized set of component rules and responsive breakpoints, replacing the stacked theme overrides.
- Slimmer sidebar, quieter background treatment, consistent borders and spacing, balanced text sizes, compact status metrics, and smaller game rows.
- Preserved the landscape backdrop and blue glass visual identity, with readable opaque backgrounds when blur is unavailable.
- Moved the Settings heading above the update card. Kept the updater mounted across navigation to preserve an active update operation.
- Aligned the update action with the card heading. Routine check failures use a compact message; the raw error remains available under a collapsed Technical details disclosure. Update consent, progress, errors, and signature verification remain.
- Added reduced-motion and reduced-transparency styling and retained keyboard focus indicators.
- Gaming features, launch behavior, font isolation, diagnostic and repair modules, backup/rollback behavior, and the existing updater signing key are preserved.

## Executed verification

- 92 Rust tests passed.
- 28 React tests passed, including the new regression confirming raw update errors remain in a collapsed disclosure.
- Five Node contract/startup tests passed.
- Seven disposable-game-fixture tests, five build-gate tests, and five release-manifest tests passed.
- Total: 142 executable tests, plus 22 source structure checks.
- TypeScript checking and Vite production build passed.
- Actual optimized Rust/Tauri compilation and native amd64 DEB packaging passed.
- The exact DEB signature and signed version were verified; modified package bytes were rejected.
- The DEB upgraded the previous installed version successfully inside the disposable build environment.

## Verification limits

The user's supplied screenshot informed this work. The refreshed native UI has not been visually rendered or inspected on the target desktop in this environment; screenshot parity, layout at every window size, live-game launch, and gameplay are not claimed. Please inspect the installed UI on the target PC.

The screenshot's update error is not resolved by a visual cleanup. GitHub source publication, signing-secret setup, and release publication are still required before the configured update endpoint can serve a release. No GitHub push or live updater installation is claimed for this build.

The DEB remains an amd64 Ubuntu 24.04-compatible build requiring glibc 2.39 or newer, not a verified universal Linux package.

## Backup and source

The complete archive includes source, a Git bundle, installer and signature, updater manifest, verification logs, and a CRC-verified v0.6.5 source-and-installer backup. It contains no private signing key; retain the separate maintainer key backup supplied with v0.6.5.
