# Proton Medic v0.6.2 — startup diagnostics

## Added

- Native checkpoints for process entry, Tauri setup, and page loading.
- An early JavaScript reporter for resource failures, errors, and unhandled promises.
- A loading message before React starts.
- A React error boundary that displays rendering failures.
- A 15-second check for a missing application DOM.

Startup messages go to stderr. The app does not upload diagnostics.

## Capture a startup log

```bash
proton-medic 2>&1 | tee ~/proton-medic-startup.log
```

Review local file paths before sharing the log.

## Validation

The release gate and four startup-reporter tests passed. The new tests cover missing DOM, present DOM, resource failures, and bounded rejection messages.
