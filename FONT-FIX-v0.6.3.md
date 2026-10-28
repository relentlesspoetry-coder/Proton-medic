# Proton Medic v0.6.3 — bundled fonts

## Fixed

Used bundled fonts to avoid blank WebKit windows on affected Zorin installations.

## How it works

- Bundle regular and bold DejaVu Sans with the application.
- Extract the fonts into a private temporary directory before GTK and WebKit start.
- Use an app-local Fontconfig configuration and a fresh private cache.
- Restore the original font environment for Steam, Wine, and Proton subprocesses.
- Remove the temporary font directory during normal shutdown.

System fonts and caches are unchanged. A crash can leave a temporary font directory. The DejaVu license is included with the package.

Set `PROTON_MEDIC_SYSTEM_FONTS=1` to use system fonts for troubleshooting. The bundled font set has limited script and emoji coverage.

## Validation

127 tests and 21 source checks passed. TypeScript checking, frontend and native compilation, DEB packaging, and package installation passed. Fontconfig discovery found the two bundled fonts without a prebuilt cache.

DEB SHA-256: `c33ba2e3500bbe6c17e75c36e6e0af4fcdd857242629387f2974c706456e5922`
