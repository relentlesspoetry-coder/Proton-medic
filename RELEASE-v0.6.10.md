# Proton Medic v0.6.10

## Performance

- Share Steam inventory snapshots across read-only views for up to 30 seconds. Scan Again refreshes immediately; repairs and launches retain fresh validation.
- Share diagnostic results between startup analysis and Proton recommendations.
- Display large game lists in pages of 40 and load artwork near the visible area. Search and filters include the entire library.
- Use indexed lookups for issue filtering and log-monitor inventory cleanup. Unchanged polling results no longer redraw game cards.
- Write runtime output incrementally to bounded logs, with one chronological tail rewrite when a session ends.

## Fixed

- Preserve profile renames and list membership changes made just before a launch. If executable, prefix, or runtime settings change during launch preparation, the launch stops and asks for a refresh.
- Keep a Windows profile busy while observed game descendants are running, including after their launcher exits. Process identities prevent unrelated reused PIDs from keeping a session active.
- Finish output capture without waiting indefinitely on abandoned pipes after the observed process family ends. No processes are killed automatically.
- Capture Windows runtime warnings in bounded stdout and stderr logs instead of an unlimited duplicate Proton log. Existing logs are preserved. Each output log retains up to 2 MiB per session.

## Interface

- Place automatic log-fix controls first in Settings, above privacy information and technical details.
- Keep the blue glass styling, logo, update button, backups, restore actions, and game management controls.

## Update

Choose **Check for updates** at the top right, approve installation, then close and reopen Proton Medic.

Requirements: Ubuntu 24.04-compatible amd64 Linux, glibc 2.39 or newer, GTK 3, and WebKitGTK 4.1.
