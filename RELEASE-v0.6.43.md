# Proton Medic v0.6.43

- Closing Proton Medic while its Windows games or installers are running now offers **Keep Open** or **Force Quit** instead of an informational dead end.
- Force Quit warns about lost unsaved progress and interrupted installations, then stops the reviewed Proton Medic sessions and their child processes before exiting. Stubborn processes are forcibly stopped after a short grace period. Steam and unrelated apps are not targeted.
- Canceling or dismissing the prompt leaves games running. Repeated close requests share one confirmation, and new work cannot slip in during shutdown.
- Fix portable native dialogs by bundling the GTK icon and MIME data they need on clean Linux systems.

Older sessions without force-stop support, unfinished configuration changes or downloads, and incomplete session cleanup may still need to finish before closing. Existing profiles, game files, and prefixes are preserved.

DEB: Ubuntu 24.04-compatible amd64 Linux. AppImage: x86_64 Linux with glibc 2.35 or newer and working graphics drivers.
