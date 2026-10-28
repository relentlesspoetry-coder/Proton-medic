# Build Proton Medic on Linux

## Requirements

- Ubuntu 24.04-compatible x86_64 development system
- Rust 1.99.1
- Node.js 22 or newer
- GTK 3 and WebKitGTK 4.1 development libraries

Install the native dependencies:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libwebkit2gtk-4.1-dev \
  curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

## Build and test

```bash
rustup toolchain install 1.99.1 --profile minimal
rustup override set 1.99.1
npm ci
bash scripts/build-zorin.sh --check
CARGO_BUILD_JOBS=2 bash scripts/build-zorin.sh --build
```

The build gate checks prerequisites, runs the frontend and Rust suites, checks launcher fixtures and updater manifests, builds the frontend, and packages the native application as a DEB. Cargo uses the lockfile, and `npm ci` installs the locked JavaScript dependencies.

Tests use temporary Steam-style fixtures and fake launchers. They do not require Steam credentials or modify an installed game library.

The DEB is written to `src-tauri/target/release/bundle/deb/`. It requires glibc 2.39 or newer, GTK 3, and WebKitGTK 4.1.

## Development

- `npm run dev`: frontend development server
- `npm run typecheck`: TypeScript checking
- `npm test`: frontend, startup, and command-contract tests
- `npm run tauri -- dev`: desktop development
- `cargo test --locked --manifest-path src-tauri/Cargo.toml`: Rust tests

## Release packages

See [GITHUB-RELEASES.md](GITHUB-RELEASES.md) for signing and publishing. The current distribution format is an amd64 DEB. SteamOS needs a separate packaging path; do not disable its read-only system protection to install this package.
