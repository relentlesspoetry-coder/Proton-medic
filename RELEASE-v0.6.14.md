# Proton Medic v0.6.14

## Added

- Portable AppImage download for Steam Deck and other x86_64 Linux desktops. GTK, WebKitGTK, and application fonts are bundled.
- Separate signed update packages for DEB and AppImage installations, using the existing trusted signing key.
- Portable package checks in a clean desktop container without system GTK, WebKitGTK, or FUSE.

## Fixed

- Game shortcuts use the permanent AppImage path instead of a temporary mounted executable.
- Automatic update activation opens the updated AppImage from its permanent location.
- Game launches and host utilities do not inherit the AppImage's private library and plugin paths.

## Install on Steam Deck

Switch to Desktop Mode, download the AppImage, and move it to a permanent folder such as `~/Applications`. Enable **Is executable** under **Properties → Permissions**, then open it. Keep that folder writable for updates. SteamOS system protection stays enabled.

If FUSE is unavailable, start the file from Konsole with `APPIMAGE_EXTRACT_AND_RUN=1` before its quoted path. GTK and WebKitGTK do not need to be installed separately.

## Requirements

The AppImage targets x86_64 Linux with glibc 2.35 or newer and working system graphics drivers. Steam or a configured Proton/Wine runtime is needed for the corresponding game features. The Ubuntu 24.04-compatible DEB remains available; install it through `apt` to resolve its dependencies.
