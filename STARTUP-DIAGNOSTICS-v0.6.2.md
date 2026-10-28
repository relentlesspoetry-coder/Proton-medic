# Proton Medic 0.6.2 — blank-window diagnostic build

This build investigates the blank native window reported on Zorin OS 18.1,
Wayland, AMD Radeon hardware and WebKitGTK 2.52.6. The user tested DMABUF,
compositing and temporary WOFF filtering workarounds without success. The root
cause is still unknown; this build is not advertised as a confirmed fix.

## Changes

- Native stderr checkpoints for process entry, Tauri setup and page-load events.
- An early, classic JavaScript startup reporter independent of the React module.
- Reports JavaScript errors, failed resources and unhandled promises to stderr
  through a bounded native command. No network telemetry or persistent log file
  is created by the application; terminal redirection is under user control.
- A plain HTML loading message appears before React starts. A React error
  boundary presents rendering failures instead of silently leaving an empty root.
- A 15-second check distinguishes missing application DOM from DOM present but
  potentially not painted. It does not claim that visible rendering succeeded.
- All prior features, blue glass styling, and repair consent controls retained.
  No permanent graphics or font environment overrides are introduced.

## On the affected computer

Close all previous Proton Medic windows. Install the supplied 0.6.2 DEB and run:

```sh
proton-medic 2>&1 | tee ~/proton-medic-0.6.2-startup.log
```

Wait 20 seconds, then send the terminal output and whether the loading message,
error message, main interface, or blank window is visible. Error messages may
include local file paths; review before sharing.

## Verification

The normal release gate and four new startup-reporter tests were executed;
see verification/release.log and verification/startup-tests.log. The new tests
cover missing DOM, present DOM without claiming paint success, resource failure
reporting, and bounded rejection messages. They run in Node's VM and do not
substitute for native WebKit testing. A real desktop reproduction remains needed.
The archive preserves the previous complete v0.6.1 delivery as a backup.

This is an amd64 Ubuntu 24.04-class DEB, not a cross-distro validated release.
