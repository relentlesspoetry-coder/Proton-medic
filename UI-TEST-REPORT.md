# Proton Medic v0.3.0 — Browser Smoke Test Report

- **17 / 17 passed** with Chromium, TSX transpilation, and synthetic Tauri IPC responses.
- **Scope:** actual application React components and CSS, simulated Rust backend. All test game data is synthetic.
- **Covered:** approved home UI; scan on start; search/filter; game detail evidence; Fixes page; repair preview and explicit checkbox; prefix save-data risk warning; Steam-running blocker; confirmed repair invocation; backup listing and restore consent; confirmed restore invocation; system/Vulkan page; settings discovery; rescan; backend failure; empty Steam library; runtime exception checks.
- **Not covered:** Rust compilation, Rust unit-test execution, native Tauri bridge, actual Steam library mutation, real backup/restore against a filesystem, driver and Vulkan execution.
- **Rust tests authored (not executed):** 6 tests in `src-tauri/src/repairs.rs`, covering backup ID validation, data-preserving archive, symlink rejection, and preservation of newer data during restore.
- **Environment limitations:** Rust/Cargo and GTK/WebKit development dependencies unavailable. React test harness uses an older offline React compatibility shim rather than the production React 18 runtime.
- **Required before live use:** `npm install && npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run tauri dev`, then manually test archival and recovery on a disposable game under Zorin OS.

The browser checks establish UI behavior only. They **do not establish the safety or correctness of native filesystem mutations**.
