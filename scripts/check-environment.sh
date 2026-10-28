#!/usr/bin/env bash
set -euo pipefail
printf 'Proton Medic build environment\n\n'
for cmd in node npm rustc cargo pkg-config; do
  if command -v "$cmd" >/dev/null 2>&1; then
    printf '[OK] %-13s %s\n' "$cmd" "$(command -v "$cmd")"
  else
    printf '[MISSING] %-13s\n' "$cmd"
  fi
done
printf '\n'
if command -v pkg-config >/dev/null 2>&1; then
  if pkg-config --exists webkit2gtk-4.1; then
    printf '[OK] WebKitGTK 4.1\n'
  else
    printf '[MISSING] WebKitGTK 4.1 development package\n'
  fi
fi
