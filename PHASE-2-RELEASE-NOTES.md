# Proton Medic v0.2.0 — log analysis and system checks

## Added

- Find Proton logs by installed Steam App ID.
- Read bounded log tails and display short diagnostic excerpts.
- Recognize Vulkan initialization errors, device loss, missing shared libraries, Wine exceptions, DLL failures, and memory errors.
- Show log timestamps and separate missing, unreadable, no-match, and review states.
- Display OS, kernel, CPU architecture, GPU IDs, Vulkan ICD manifests, and vulkaninfo availability.
- Run an optional six-second `vulkaninfo --summary` probe.
- Add diagnostics to game details and a dedicated system workspace.
- Add seven diagnostic-engine tests.

Log analysis inspects up to 2 MiB from files no larger than 24 MiB. It reads standard Proton log locations; custom `PROTON_LOG_DIR` paths are outside this version.

Library scanning, compatibility-tool discovery, search, and navigation remain available. This version makes no repair changes.
