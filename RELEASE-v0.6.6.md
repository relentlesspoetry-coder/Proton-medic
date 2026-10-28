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

GitHub source publication, signing-secret setup, and release publication were completed on 2026-10-28. Live updater installation on the target desktop remains unverified; the user's installed v0.6.5 was left in place for that test. A browser download check of the stable latest.json URL was blocked by the browser download approval gate, so successful live endpoint retrieval is not claimed.

The DEB remains an amd64 Ubuntu 24.04-compatible build requiring glibc 2.39 or newer, not a verified universal Linux package.

## Backup and source

The complete archive includes source, a Git bundle, installer and signature, updater manifest, verification logs, and a CRC-verified v0.6.5 source-and-installer backup. It contains no private signing key; retain the separate maintainer key backup supplied with v0.6.5.

## GitHub publication verification — 2026-10-28

- [Signed release run 40770005789](https://github.com/relentlesspoetry-coder/Proton-medic/actions/runs/40770005789) completed successfully from commit `1eb4fefb65d201f6ac8b16adaca41267724b5457`.
- The release gate reran all 142 executable tests and 22 source structure checks, TypeScript/Vite production build, optimized native compilation, and amd64 DEB packaging successfully.
- The existing private key was configured as an Actions secret. The exact DEB signature and signed version passed verification against the embedded public key; altered package bytes were rejected.
- [v0.6.6](https://github.com/relentlesspoetry-coder/Proton-medic/releases/tag/v0.6.6) is the published stable release, with `latest.json`, `proton-medic_0.6.6_amd64.deb`, its `.sig`, and `SHA256SUMS` confirmed in the release API.
- Published DEB SHA-256: `1694d8bdc9a7bddaf0db1ea37b5fe0e1c0607b8fdfdd5520cfb18df1b533a0e1`.
- Source before release setup is preserved on `backup/v0.6.6-before-release-setup` at `fffdd4526dda879f92e8febbc4e2fee1537a9184`. Application source, fonts, CSS, and public signing key were unchanged by release setup.
- The local v0.6.5 DEB payload was read successfully; its SHA-256 is `d8598ca239b4a12d2ae4b662f57457659ea5f8165832ce21048b9348225aa83a`. The installed v0.6.5 binary contains the same public key and configured stable endpoint as v0.6.6.
- The earlier disposable-environment installation result above is historical. This publication session did not install the release or verify desktop UI, privilege prompts, live updating, or real-game launching.
