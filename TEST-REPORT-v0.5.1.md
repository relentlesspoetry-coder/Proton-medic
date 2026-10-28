# Proton Medic v0.5.1 — test results

Version-specific results.

| Check | Result |
| --- | --- |
| Rust tests | 60 passed |
| React interaction tests | 9 passed |
| Python launcher fixtures | 7 passed |
| Build-gate tests | 5 passed |
| Launch-status contract test | 1 passed |
| Source checks | 20 passed |
| TypeScript and Vite build | Passed |
| Native build and DEB packaging | Passed |
| Package installation and payload comparison | Passed |
| Shared-library resolution | Passed |
| Desktop and real-game tests | Not completed |

## Coverage

Read-only startup baseline, settled-log detection, Steam-closed deferral, one attempt per game, non-overlapping polling, consent, cleanup, and rollback receipt delivery.

Total: 82 executable tests and 20 source checks. Rust tests use temporary fixtures; React tests use mocked IPC.

DEB SHA-256: `40e2be44dbe70d8a5f2cab9ac4f6eabaa154ff250205ed513c07976ac23d9c3d`
