# Proton Medic v0.6.1 — reliability improvements

## Fixed

- Blocked reuse of interrupted Windows-game profiles until a reboot clears the old run.
- Added per-profile locks to prevent conflicting changes from separate app instances.
- Retained the most recent 2 MiB of runtime output, including late failure messages.
- Separated missing, unreadable, partial, and recognized-error log states.
- Restored access to logging rollback when reopening a game's Fix & Test panel.
- Kept failed completion receipts in memory and retried saving on library refresh.
- Enforced a six-second Vulkan output deadline, including open descendant pipes.

## Changed

Moved the blue glass theme into its own stylesheet. Runtime output snapshots are saved every 256 KiB and at completion.

Logging restore and failed-completion receipts last for the current app session. Configuration backups remain on disk.

## Validation

119 tests and 21 source checks passed. TypeScript checking, frontend compilation, native compilation, DEB packaging, and package installation passed.

DEB SHA-256: `bc861784a6c61b1b86bcd9bce0d7ced2162267775f02ddc7bb3bbe7d824c03ba`
