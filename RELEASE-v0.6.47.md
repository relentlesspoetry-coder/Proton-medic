# Proton Medic v0.6.47

- Stage AppImage updates beside the installed application and atomically replace it only after the complete replacement is written, synchronized, and executable. An interruption during staging leaves the current application intact.
- Avoid failed update attempts caused by unavailable temporary or cache directories when the application folder is writable.
- Keep signature and signed-version verification unchanged. Native interruption and failure regressions now gate releases.

These updater improvements take effect once this version is installed. Game files, Windows prefixes, and Steam settings are unchanged.

DEB: Ubuntu 24.04-compatible amd64 Linux. AppImage: x86_64 Linux with glibc 2.35 or newer and working graphics drivers.
