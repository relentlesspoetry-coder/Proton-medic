#!/usr/bin/env bash
# Proton Medic — local Debian/Ubuntu/Zorin native build gate.
# No root privileges, no apt installs, no real Steam launches, no repair actions.
set -Eeuo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

usage() {
  cat <<'USAGE'
Proton Medic native validation (Zorin OS / Ubuntu / Debian)

Usage:
  ./scripts/build-zorin.sh --check   Inspect prerequisites, do not build
  ./scripts/build-zorin.sh --build   Run all tests, build a DEB only if they pass
  ./scripts/build-zorin.sh --help    Show this help

Install required Linux packages (manual, explicit):
  sudo apt update
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev pkg-config

Install a recent Rust toolchain using official https://rustup.rs/ and Node.js 20+.
Then in this project directory run:
  npm install
  ./scripts/build-zorin.sh --build

The script neither installs packages nor modifies Steam games or compatibility settings.
USAGE
}

mode="${1:---check}"
case "$mode" in
  --help|-h) usage; exit 0 ;;
  --check|--build) ;;
  *) usage >&2; exit 64 ;;
esac

failures=0
need_bin() {
  local name="$1"
  if command -v "$name" >/dev/null 2>&1; then
    printf '[PASS] %-22s %s\n' "$name" "$(command -v "$name")"
  else
    printf '[BLOCKED] Missing executable: %s\n' "$name"
    failures=$((failures+1))
  fi
}
need_lib() {
  local name="$1"
  if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists "$name"; then
    printf '[PASS] %-22s %s\n' "$name" "$(pkg-config --modversion "$name")"
  else
    printf '[BLOCKED] Missing native pkg-config library: %s\n' "$name"
    failures=$((failures+1))
  fi
}

echo '[INFO] Read-only build prerequisites; no Steam files will be accessed.'
for bin in node npm python3 cargo rustc pkg-config gcc dpkg-deb; do need_bin "$bin"; done
# The locked Tauri dependency requires Rust 1.90 or newer.
if command -v rustc >/dev/null 2>&1; then
  rust_version=$(rustc --version | awk '{print $2}')
  if [[ "$rust_version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+) &&
        ( ${BASH_REMATCH[1]} -gt 1 || ( ${BASH_REMATCH[1]} -eq 1 && ${BASH_REMATCH[2]} -ge 90 ) ) ]]; then
    printf '[PASS] Rust version suitable for source: %s\n' "$rust_version"
  else
    printf '[BLOCKED] Rust version %s is too old; use rustup to install >=1.90\n' "$rust_version"
    failures=$((failures+1))
  fi
fi
for lib in gtk+-3.0 webkit2gtk-4.1 javascriptcoregtk-4.1 libsoup-3.0; do need_lib "$lib"; done
if [[ -x node_modules/.bin/vite && -x node_modules/.bin/tauri && -x node_modules/.bin/tsc ]]; then
  echo '[PASS] Frontend dependencies installed locally'
else
  echo '[BLOCKED] Frontend dependencies absent: run npm install in this project'
  failures=$((failures+1))
fi
if [[ "$mode" == '--check' ]]; then
  if (( failures )); then
    echo "[RESULT] BLOCKED: $failures missing prerequisite(s). No build attempted."
    echo '[NEXT] See ./scripts/build-zorin.sh --help for installation commands.'
    exit 2
  fi
  echo '[RESULT] Native build prerequisites appear present; run --build to compile and test.'
  exit 0
fi
if (( failures )); then
  echo "[RESULT] BLOCKED: $failures missing prerequisite(s). No tests or native build attempted."
  echo '[NEXT] See ./scripts/build-zorin.sh --help for installation commands.'
  exit 2
fi

# Tests and build work only with fake fixtures and project source. Never run
# commands that launch a real Steam game or modify compatibility settings.
echo '[RUN] Frontend syntax and structural checks'
npm test
echo '[RUN] Disposable filesystem and fake-launcher protocol checks (Python)'
python3 scripts/test-disposable-fixtures.py
echo '[RUN] Native build gate regression checks (Python)'
python3 scripts/test-build-gate.py
echo '[RUN] Updater release manifest tests (Python)'
python3 scripts/test-release-manifest.py
echo '[RUN] Automatic release policy tests (Python)'
python3 scripts/test-release-ready.py
echo '[RUN] Full native Rust suite (includes fake Steam and rollback tests)'
cargo test --locked --manifest-path src-tauri/Cargo.toml -- --test-threads=2
echo '[RUN] TypeScript typecheck and Vite production build'
npm run build
echo '[RUN] Tauri native DEB build (no AppImage/download-based bundling)'
npm run tauri -- build --bundles deb -- --locked

echo '[RUN] Native desktop shortcut integration tests (fake Wine, disposable profile)'
python3 scripts/test-windows-shortcut.py src-tauri/target/release/proton-medic

shopt -s nullglob
packages=(src-tauri/target/release/bundle/deb/*.deb)
if (( ${#packages[@]} == 0 )); then
  echo '[FAIL] Tauri command returned successfully but no DEB was produced.' >&2
  exit 1
fi
for pkg in "${packages[@]}"; do
  dpkg-deb --info "$pkg" >/dev/null
  echo "[PASS] Built package: $pkg"
  sha256sum "$pkg"
done
echo '[PASS] Rust tests, frontend build, and DEB packaging completed successfully.'
