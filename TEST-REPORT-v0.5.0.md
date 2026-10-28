# Proton Medic v0.5.0 — test results

Version-specific results.

| Check | Result |
| --- | --- |
| Rust tests | 56 passed |
| React interaction tests | 5 passed |
| Python launcher fixtures | 7 passed |
| Build-gate tests | 5 passed |
| Launch-status contract test | 1 passed |
| Source checks | 19 passed |
| TypeScript and Vite build | Passed |
| Native build and DEB packaging | Passed |
| Package installation and payload comparison | Passed |
| Shared-library resolution | Passed |
| Desktop and real-game tests | Not completed |

## Coverage

Fix eligibility, consent, one-attempt limits, failed launches, selective rollback, conflict rejection, backup preservation, and UI session recovery.

Total: 74 executable tests and 19 source checks. Rust tests use temporary files and fake launchers; React tests use mocked IPC.

DEB SHA-256: `02409f20b76db6999420f0970ec165c2c652c0cc2b5fbf6996f2edb4c02b53e7`
