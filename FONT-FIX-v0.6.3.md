# Proton Medic 0.6.3 — app-local font startup fix

The user verified that a single-font, fresh-cache Fontconfig environment makes
Proton Medic render on Zorin OS 18.1, Wayland, AMD Radeon hardware and WebKitGTK
2.52.6. The exact offending system font/cache remains unidentified.

This release applies font isolation before Tauri/GTK/WebKit initialization.
Regular and bold DejaVu Sans are embedded, extracted to a unique private temporary
directory, and selected through an app-local FONTCONFIG_FILE. The configuration
includes only those fonts and a fresh private cache. Fontconfig populates the
cache on first use; startup does not run a shell or depend on fc-cache. Tests
verify fc-list discovers exactly the two bundled fonts with no prebuilt cache.
System fonts, settings and caches are not edited. Normal shutdown removes the
private directory; a crash may leave a small temporary directory behind.

Steam and Wine/Proton subprocesses restore the original FONTCONFIG_FILE value,
or remove the override if none existed. They do not inherit the app's font
restriction. A deliberate opt-out is available with PROTON_MEDIC_SYSTEM_FONTS=1.
This selects the normal user font environment again and can reproduce the known
blank-window problem on affected systems.

DejaVu license notices are included in the source and installed package resources.
The limited UI font set may not cover every script or emoji in game names. Future
language coverage should use additional verified bundled fonts, not reintroduce
unrestricted host font scanning. No graphics overrides or sandbox weakening added.

The normal application launcher now uses this startup path; no terminal font
setup should be necessary. The previous user-tested workaround is evidence for
the approach, not verification of this new installer on the user's machine.

Validation output is included under verification/. Four native regressions cover
private config/cache generation and cleanup, XML escaping, game environment
restoration, and actual Fontconfig discovery without a cache rebuild. Existing
startup diagnostics remain for troubleshooting. The old structural test was
updated to follow the new environment-restoring command helper and still checks
explicit process arguments without a shell.

This DEB remains for Ubuntu 24.04-class amd64 Linux (glibc >=2.39). Steam Deck,
other distros, real Windows installers and universal game compatibility are not
newly verified by this release. Prior functionality and blue glass UI are retained.

Backup note: the local previous source ZIP was found truncated. All 116 complete
prior source entries were recovered from its local records and CRC-verified, then
repackaged with the existing v0.6.2 installer in a new validated backup. The
truncated original bytes were retained separately; the new backup is not claimed
to be byte-identical to that archive.

Final validation: 89 Rust tests, 21 React tests, 5 Node tests and 12 Python tests
passed (127 total), plus 21 structural checks. TypeScript, Vite and optimized
Rust/Tauri compilation succeeded. The DEB installed with exit 0 and dpkg reports
install ok installed 0.6.3. Its installed binary matches the extracted payload;
the DejaVu license is included in package resources. Native GUI verification of
this installer on the affected user's desktop is still pending.

DEB SHA-256: c33ba2e3500bbe6c17e75c36e6e0af4fcdd857242629387f2974c706456e5922
