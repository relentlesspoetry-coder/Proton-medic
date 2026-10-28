# Proton Medic v0.6.0 — test results

Version-specific results.

| Check | Result |
| --- | --- |
| Rust tests | 80 passed |
| React interaction tests | 18 passed |
| Python launcher fixtures | 7 passed |
| Build-gate tests | 5 passed |
| Launch-status contract test | 1 passed |
| Source checks | 21 passed |
| TypeScript and Vite build | Passed |
| Native build and DEB packaging | Passed |
| Package installation and payload comparison | Passed |
| Shared-library resolution | Passed |
| Windows console fixture compilation | Passed |
| Wine fixture execution | Failed before Windows execution: wineserver socket permission error |
| Graphical installer and real-game tests | Not completed |

## Coverage

PE format checks, managed-drive containment, symlink rejection, profile identity, installer consent, process arguments, runtime environment, bounded output, missing runtimes, and executable selection.

Total: 111 automated tests and 21 source checks. The Wine execution failure is excluded from the passing total. Runtime fixtures simulate installer and game processes; React tests use mocked IPC and file dialogs.

DEB SHA-256: `a52283c879ebbf6163d6d546615a03245281dbf701ad5c670d9541a43403ff34`
