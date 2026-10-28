# Proton Medic v0.4.3 — Test report

## Actually executed in this session

- 18 / 18 TypeScript syntax and static source/configuration checks: **PASS**.
  These are not equivalent to `tsc --noEmit` or Rust compilation.
- 7 / 7 disposable fake-Steam launcher protocol checks in Python: **PASS**.
  These do **not** execute the Rust launch code.
- 27 / 27 Chromium UI integration smoke tests using the actual TSX components
  with a simulated Tauri backend: **PASS**. These do not use React 18/Tauri natively.
- `bash -n` on the build scripts: **PASS**.
- Build gate preflight: **BLOCKED as expected**, missing native prerequisites.
- 4 / 4 build gate regression tests: **PASS**. These validate the command-line
  preflight; they do not compile the application.

## Not executed

- `cargo test`: Cargo/rustc absent; Rust test suite not compiled.
- `npm run build`: local npm package dependencies unavailable.
- Native `tauri build --bundles deb`: missing Rust, GTK/WebKit, and npm deps.
- Real Steam game launch or compatibility configuration editing: not attempted.

## Build preflight blockers in this sandbox

1. `cargo`
2. `rustc`
3. `gtk+-3.0`
4. `webkit2gtk-4.1`
5. `javascriptcoregtk-4.1`
6. `libsoup-3.0`
7. local npm dependencies (`node_modules`)

The above list reflects the original preflight; the new gate also verifies
`dpkg-deb` and the Rust toolchain version if present. This release is **source
only** and must not be represented as an installer.
