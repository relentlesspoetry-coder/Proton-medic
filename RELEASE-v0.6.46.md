# Proton Medic v0.6.46

- Keep host media plugins available to games launched from the AppImage. Bundle-only GStreamer paths no longer override the host's normal plugin discovery.
- Resolve Proton runtime ownership from Steam installations and library mappings, including external drives and symlinks. Flatpak-only or ambiguously shared tools are not launched on the host, and native tools use their actual Steam installation.
- Preserve unrelated Steam configuration fields when changing a Proton selection. Ambiguous duplicate game mappings or compatibility names are rejected before writing.
- Restore logging options correctly when the original launch options contain tabs. Original configuration backups are retained.

Game files, Windows prefixes, and existing backups are preserved.

DEB: Ubuntu 24.04-compatible amd64 Linux. AppImage: x86_64 Linux with glibc 2.35 or newer and working graphics drivers.
