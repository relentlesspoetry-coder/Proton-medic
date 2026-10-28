# Proton Medic v0.4.0 — test results

Version-specific results.

| Check | Result |
| --- | --- |
| Browser UI tests | 27 passed |
| Source and command-contract checks | 18 passed |
| TOML parsing and shell syntax | Passed |
| Native preflight | Stopped at missing prerequisites |
| Rust tests, production frontend build, and native packaging | Not run |

## Coverage

Launch consent, App ID and library arguments, bounded observation, fresh-log states, backend failures, and player confirmation. Six Rust launch tests were added for error classification, fresh logs, fake launchers, and approval checks.

Browser tests used simulated Tauri responses.
