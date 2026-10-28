# Proton Medic 0.2.0 — Phase 2 Source Release

Date: 2026-10-28

## Added

- Read-only game log discovery by installed Steam App ID.
- Bounded trailing-log parser (2 MiB inspected, 24 MiB per-file maximum) with short excerpts.
- Severity-labelled, evidence-based signatures: Vulkan init / device loss, missing shared objects, Wine exceptions, DLL/module load failures, memory allocation failures, and informational anti-cheat references.
- Log last-modified time and clear no-log, unreadable, no-match, and review states.
- Host system diagnostics: OS, kernel, CPU architecture, GPU IDs, Vulkan ICD manifests, and availability of vulkaninfo.
- Optional six-second, explicitly invoked `vulkaninfo --summary` probe with bounded output.
- Diagnostics workspace, game details integration, log capture help, and new read-only indicators.
- Seven synthetic diagnostic-engine unit tests, in addition to Phase 1 scanner tests.

## Preserved

- All Phase 1 library scanning, prefix inspection, compatibility tool discovery, game search, navigation, and warning screens.
- Local-only, unprivileged, no-repair-first policy.

## Known limitations

- No custom PROTON_LOG_DIR support; no system-wide/Flatpak container GPU validation; no confirmed root-cause detection; no repairs.
- Needs first native build and runtime validation on Zorin/Steam Deck.
- The source release includes tests but they could not be executed here due to missing Cargo and inaccessible npm registry.

## Version handling

Full independent source ZIP of v0.2. Prior v0.1 ZIP remains the backup; never overwrite it while validating v0.2.
