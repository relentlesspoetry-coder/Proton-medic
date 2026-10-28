# Proton Medic v0.5.2 — test results

Version-specific results.

| Check | Result |
| --- | --- |
| Rust tests | 71 passed |
| React interaction tests | 12 passed |
| Python launcher fixtures | 7 passed |
| Build-gate tests | 5 passed |
| Launch-status contract test | 1 passed |
| Source checks | 20 passed |
| TypeScript and Vite build | Passed |
| Native build and DEB packaging | Passed |
| Package installation and payload comparison | Passed |
| Shared-library resolution | Passed |
| Desktop and real-game log generation | Not completed |

## Coverage

Launch-option preservation, logging assignments, minimal edits, backup/enable/restore operations, account resolution, ambiguous configuration, symlink rejection, consent, and queued setup.

Total: 96 executable tests and 20 source checks. Rust tests use temporary Steam account configuration; React tests use mocked IPC.

DEB SHA-256: `c8cddc7ea6ef5a58ebe150ff49d9581d789dae9efb7a9b43dd7745e5fc919bbb`
