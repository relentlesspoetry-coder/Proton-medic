# Proton Medic v0.6.16

- Added Launch history to each Windows game card, showing the latest 20 game and installer runs.
- Records launches from both Proton Medic and its desktop shortcuts, including the runtime used, start time, elapsed time, and runtime exit result.
- Saves recognized error summaries from each run’s captured logs and identifies partial logs, failed starts, unknown exits, and runs interrupted by a system restart.
- Loads history only when expanded. Saved summaries avoid repeatedly scanning logs while browsing the library.
- Preserves existing game files, prefixes, and logs. Earlier runs without history metadata are kept without inventing timestamps or exit results.

Open **Windows Games → Launch history** on a game card. Runtime exit codes describe the launcher’s result; they do not verify gameplay. Signed DEB and AppImage updates remain available.
