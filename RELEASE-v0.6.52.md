# Proton Medic v0.6.52

- Retry temporary update-server failures up to three times within the existing 20-second update-check limit.
- Include underlying connection and timeout causes in the update check's technical error details.
- Add regressions for temporary errors, retry exhaustion, permanent errors, malformed metadata, TLS failures, and duplicate check clicks. Update signatures and installation safeguards remain unchanged.

DEB: Ubuntu 24.04-compatible amd64 Linux. AppImage: x86_64 Linux with glibc 2.35 or newer and working graphics drivers.
