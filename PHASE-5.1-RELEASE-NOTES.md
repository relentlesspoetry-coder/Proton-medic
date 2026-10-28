# Proton Medic 0.5.1 — automatic game log reader

The app now monitors Steam Proton logs every ten seconds while its window is
open. No log upload or repeated Scan button is required. The monitor checks
metadata first and reads bounded log tails when files change. Polls do not
intentionally overlap; a native lock also rejects overlapping IPC requests.

## Automatic fixes

Enable **Automatically apply supported fixes** in the log-reader section or
Settings. This is a new opt-in preference, disabled by default rather than
silently inheriting the old automatic-selection setting.

1. Existing logs are read as a startup baseline. Use Fix & Test for an existing
   finding; automatic changes respond to evidence written after monitoring starts.
2. A changed log is read automatically. The monitor waits for an unchanged poll
   before considering a fix, rather than modifying settings as the log grows.
3. Fresh exception/device-loss findings can select the existing backed-up Proton
   alternative recipe. Other findings are displayed for review.
4. With automatic fixes enabled, supported changes wait until Steam is fully
   closed, then apply automatically. No additional per-change click is required.
5. The result and backup path are displayed. Open the game's Fix & Test section
   to launch-test, keep, or roll back the selection. Monitoring does not launch
   games without a separate launch approval.

There is one automatic attempt per game per app session, including failed
attempts. More log writes do not rearm it. One change is processed per polling
cycle. Disabling the option stops future automatic requests; a write already
in progress can finish. Monitoring continues read-only.

The existing manual Proton-selection button and all manual diagnostics,
launch-test, prefix/cache backup, restoration, and Fix & Test controls remain.
The old scan-triggered automatic-selection effect is replaced by this monitored,
backed-up workflow so two automatic writers do not compete.

## Limits

- Steam must generate logs. For standard Proton logs set `PROTON_LOG=1 %command%`
  in the game's Steam launch options. Custom log destinations are not discovered
  by this version; standard native/Flatpak Proton log locations are used.
- This does not read every game's proprietary log format or support standalone
  game environments yet. Only the existing supported Proton-selection recipe is
  automatically applied; it does not install system packages or DLLs, reset
  prefixes, download runtimes, or bypass anti-cheat.
- Keep the app open until keeping or rolling back a fix. Backups persist, but
  automatic rollback receipts remain session-local as in 0.5.0.
- A stable poll is not proof that a game exited. Configuration edits additionally
  require Steam closed and are revalidated by the existing native safety checks.
- An applied selection is not proof of playable gameplay.
- Distribution remains the tested Ubuntu 24.04-class amd64 DEB, not yet a
  universal Linux or SteamOS installer.

The full previous v0.5.0 delivery is preserved in the new archive's backup.
