# Proton Medic v0.6.51

- Expire resource readings from their capture time, so delayed or repeated responses cannot keep stale CPU and memory values on screen.
- Skip corrupt Steam icon files and continue to valid cached artwork.
- Regenerate damaged Windows game icons from the unchanged executable while preserving valid caches and refusing symlink replacements.
- Add regression coverage for sample expiry, supported PNG/JPEG formats, bounded decoding, icon fallback, and cache recovery.

DEB: Ubuntu 24.04-compatible amd64 Linux. AppImage: x86_64 Linux with glibc 2.35 or newer and working graphics drivers.
