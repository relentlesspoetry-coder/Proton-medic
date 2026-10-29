# Proton Medic v0.6.19

- Added troubleshooting report export in Settings. Choose Steam diagnostics, Windows game status, and optional recent Windows launch history.
- Preview the exact JSON file before saving. System details are included, and personal paths are redacted by default. Path-bearing fields are replaced in full; game names, runtime names, and selected launch timestamps remain visible.
- Raw logs, log excerpts, launch arguments, environment variables, account data, removed profiles, and resource samples are excluded.
- Reports include up to 100 Steam games, 50 visible Windows games, and 20 recent runs per Windows game. Omitted counts and unavailable records are marked.
- Saving uses the reviewed snapshot, creates a private local file, and refuses to overwrite existing files. Canceling the save dialog keeps the preview available.

Open **Settings → Create report → Generate preview**, review the contents, approve saving, and choose a new `.json` filename. Nothing is uploaded automatically. Signed DEB and AppImage updates remain available.
