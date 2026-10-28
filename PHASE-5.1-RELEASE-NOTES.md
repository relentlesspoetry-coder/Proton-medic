# Proton Medic v0.5.1 — automatic game log reader

## Added

- Monitor standard Steam Proton logs every ten seconds while the app is open.
- Check file metadata before reading bounded log tails.
- Use existing logs as a read-only startup baseline.
- Wait for changed logs to settle before considering a fix.
- Queue supported Proton-selection changes until Steam closes.
- Display the result and backup path and connect it to Fix & Test.

## Use

Enable **Automatically apply supported fixes** in the log-reader section or Settings. The option is off by default.

New supported exception or device-loss findings can trigger one backed-up Proton-selection attempt per game per app session. Open Fix & Test to approve a launch, keep the result, or roll it back.

Disabling automatic fixes stops future requests; an operation already in progress can finish. Read-only monitoring continues. Games require separate launch approval.

This version reads standard native and Flatpak Proton logs. Custom log destinations and standalone-game log formats are outside its scope. Keep the app open to retain active rollback receipts; backups persist after exit.

See [TEST-REPORT-v0.5.1.md](TEST-REPORT-v0.5.1.md) for test results.
