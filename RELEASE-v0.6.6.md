# Proton Medic v0.6.6

## Changed

- Refined the blue glass interface with a slimmer sidebar, quieter background, consistent spacing, compact status metrics, and smaller game rows.
- Consolidated theme styles and responsive breakpoints.
- Added readable opaque backgrounds when blur is unavailable.
- Moved the Settings heading above the update card and aligned the update button with its heading.
- Kept update operations active when navigating between pages.
- Simplified update errors; detailed messages are available under **Technical details**.
- Added reduced-motion and reduced-transparency styles and retained keyboard focus indicators.

Game discovery, diagnostics, launch checks, Windows installer support, backups, and rollback remain available.

## Updates

Open **Settings → App updates → Check for updates**. Review the available version and notes, then approve installation. Updates are signature-verified before installation. Close and reopen Proton Medic afterward.

## Requirements

Ubuntu 24.04-compatible amd64 Linux, glibc 2.39 or newer, GTK 3, and WebKitGTK 4.1.

## Validation

[Release workflow](https://github.com/relentlesspoetry-coder/Proton-medic/actions/runs/40770005789): 142 tests and 22 source checks passed. Frontend compilation, native compilation, DEB packaging, signature verification, and altered-package rejection passed.

## Downloads

- `proton-medic_0.6.6_amd64.deb`
- `proton-medic_0.6.6_amd64.deb.sig`
- `SHA256SUMS`
- `latest.json`

DEB SHA-256: `1694d8bdc9a7bddaf0db1ea37b5fe0e1c0607b8fdfdd5520cfb18df1b533a0e1`
