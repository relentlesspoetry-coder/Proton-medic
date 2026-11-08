# Proton Medic v0.6.48

- Recognize form-feed whitespace consistently in Steam VDF files. An unusually formatted library list or game manifest no longer hits the token limit and hides otherwise discoverable games.
- Add regression coverage for every supported ASCII whitespace separator, quoted whitespace, and external-library discovery.

This is a small scanner compatibility fix. Game files, Windows prefixes, and Steam settings are unchanged.

DEB: Ubuntu 24.04-compatible amd64 Linux. AppImage: x86_64 Linux with glibc 2.35 or newer and working graphics drivers.
