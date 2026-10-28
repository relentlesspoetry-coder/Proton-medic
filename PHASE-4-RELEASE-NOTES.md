# Proton Medic v0.4.0 — Phase 4: Launch Observation & Native Validation

## New functionality

- Explicit **Launch & Check Game** action in the existing My Games detail page.
- Native `test_game_launch` Tauri command checks that the game is in a fresh Steam scan and has an installation folder before attempting a launch.
- Uses a literal process/argument vector: native Steam (`steam -applaunch APPID`) or Flatpak Steam (`flatpak run com.valvesoftware.Steam -applaunch APPID`); no user-provided shell or launcher executable.
- Bounded observation (10 seconds in the UI, 5–45 seconds enforced by backend) checks for a *new* game-specific Proton log in the known default locations, then reuses existing diagnostic signatures.
- Distinguishes launcher error, fresh-log error, fresh-log warning, no-recognized-errors, and no-fresh-log. None of these is an automatic "game works" verdict.
- Optional **Yes, the game works** confirmation is explicitly attributed to the user for that test only. It is not persisted as a certified compatibility guarantee.
- No unattended gameplay automation, game process termination, automatic version cycling, Proton installs, repair operations, or user Steam launch-option edits.
- Disposable Rust fake-Steam launcher tests added, with temporary directories and no real Steam access.
- New `scripts/validate-native.sh` checks tools and native libraries, runs Cargo tests, builds frontend, and attempts full Tauri packaging only after prerequisites pass.

## What was retained

Existing Phase 1–3.2 scanner, diagnostic engine, Vulkan check, auto Proton recommendations and selection, archive-backup repair tools, restore workflow and opt-in settings. The clean approved home/navigation layout remains unchanged.

## Important limits

Steam's `-applaunch` command may only hand off to an existing Steam process; its exit code does not indicate game success. The launcher may continue running after the observation window and is never killed by Proton Medic. Proton logs may be absent unless logging is enabled through Steam launch options, and a clean log is not proof that gameplay works. Logs written to custom `PROTON_LOG_DIR` locations are currently out of scope.

Backend Rust tests are included but **not yet run** in the current sandbox. Do not use privileged repair or auto-edit features on important Steam setups until native build and filesystem tests pass on an actual Linux machine.
