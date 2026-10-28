# Native Linux build

The v0.6.1 validation environment is a disposable Ubuntu 24.04 x86_64
container with no host Steam library mounted. Rust/Cargo live in a separate
workspace-local toolchain. Node.js 24.22.1 comes from the execution runtime.
See RELEASE-v0.6.1.md for the actual outcome and limitations.

## Reproduce on Ubuntu 24.04

Install the native dependencies in a disposable VM/container or development machine:

```sh
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libwebkit2gtk-4.1-dev \
  curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Install Node.js 24.22.1 and Rust 1.99.1 using their official distributions.
With rustup already installed:

```sh
rustup toolchain install 1.99.1 --profile minimal
rustup override set 1.99.1
npm ci
./scripts/build-zorin.sh --check
CARGO_BUILD_JOBS=2 ./scripts/build-zorin.sh --build
```

The release gate runs frontend checks, the IPC status contract regression,
Python disposable launcher fixtures, preflight regressions, the actual Cargo
unit tests, TypeScript/Vite production compilation, and Tauri release bundling.
Cargo uses `--locked`; npm CI uses `package-lock.json`. The locked Tauri
version requires at least Rust 1.90. Only Rust 1.99.1 was tested here.

Output: `src-tauri/target/release/bundle/deb/*.deb`.

The tests do not require real games, Steam credentials, or changes to a real
Steam installation. Rust fixture tests cover discovery, diagnosis, selection,
launch observation, backup creation, and rollback. Python launcher fixtures
are independent protocol checks; they are not a replacement for Rust tests.

## Platform scope

This build targets Debian-family x86_64 Linux with GTK 3 and WebKitGTK 4.1.
A `.deb` is not a native SteamOS installer. Steam Deck SteamOS packaging and
hardware/gameplay validation remain separate work; do not modify SteamOS's
read-only system to install this package. Ubuntu/Zorin versions with older
runtime libraries need a compatible build environment and separate testing.

Historical PHASE reports describe older releases; the v0.6.1 report is the
current validation record. GitHub Actions configuration is provided, but a
remote Actions run is not implied by local validation.
