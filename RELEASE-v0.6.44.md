# Proton Medic v0.6.44

- Keep draining runtime output if a diagnostic log cannot be written, so a full disk or log I/O error does not itself close the game's output pipe. Lost log output is still reported in the completion receipt.
- Apply only the Proton version shown in the advisor's approval button. If the recommendation changes, ask the player to review it again before changing Steam settings.
- Preserve newly created logging backups and restore controls when an older status request finishes late.
- Allow a release interrupted after tag creation to retry safely, while refusing to reuse a tag that points to different source.

Game files, Windows prefixes, and existing backups are preserved.

DEB: Ubuntu 24.04-compatible amd64 Linux. AppImage: x86_64 Linux with glibc 2.35 or newer and working graphics drivers.
