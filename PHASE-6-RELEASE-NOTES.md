# Proton Medic 0.6.0 — Windows installer support

A Windows Games page adds a native .exe picker, installed runtime selection,
and an explicit installer approval. Each installation receives its own persistent
Wine/Proton prefix under the user's application data directory. The existing
charcoal/green UI and Steam discovery, diagnostics, logging setup, selection,
launch testing, backups, and rollback remain available.

## Use

1. Open Windows Games and choose a trusted Windows .exe installer.
2. Enter a game name and select an installed Wine or native Steam Proton runtime.
3. Approve execution and click Run installer. Complete the installer window,
   installing to its default C: location. Keep Proton Medic open.
4. After the installer exits, choose the installed game's .exe within that
   profile's Windows C: drive. Click Launch game and verify gameplay yourself.
5. Read runtime logs to inspect recognized diagnostic signatures. Runtime
   stdout/stderr are captured automatically for each installer and game run;
   Proton logging is also enabled. Captured streams are limited to 2 MiB each.

## Behavior and scope

- Installer and runtime paths are passed as individual process arguments, never
  interpolated into a shell command. Adjacent installer data files remain
  accessible because the working directory is the executable's parent folder.
- .exe files must have an x86/x64 Windows PE header. This is format screening,
  not a malware scan. A separate prefix is not a security sandbox: a trusted
  Windows program runs with access available to the current Linux user.
- No existing Steam prefix or default Wine prefix is modified by this flow.
  No uninstall, prefix deletion, automatic download, or destructive repair is
  introduced. Metadata and logs are retained; this release does not provide
  standalone installation rollback or automatic repair recipes.
- Installer exit zero is shown as process completion, never proof of successful
  installation. Selecting an executable and confirming playability remain
  explicit steps. Running jobs are limited to four, one per profile.
- Game records survive app restarts. A previously running process becomes
  "session-ended" with unknown process status; the app does not reclaim or kill
  it. Check open installer/game windows before launching again.
- Uses available system Wine/Wine64 or installed native Steam Proton. Runtime
  installation, runtime migration, Flatpak-only Proton, arbitrary install drives,
  .msi input, ARM Windows binaries, store-account management, and anti-cheat
  bypass are outside this release. A Wine64-only installation may lack 32-bit
  support required by particular installers.

## Verification limits

The native build and automated fixtures are documented in TEST-REPORT-v0.6.0.md.
A real Windows console fixture was compiled, but Wine execution was blocked by
this container's socket restrictions. No graphical Windows installer or actual
standalone game has been verified. This is an implementation milestone pending
Linux-desktop acceptance, not a claim of universal installer compatibility.
The supplied amd64 DEB targets Ubuntu 24.04-class systems (glibc >= 2.39);
other distributions and Steam Deck still require packaging and validation.
