#!/usr/bin/env bash
# Compatibility entry point used by v0.4.1/v0.4.2 release notes.
set -Eeuo pipefail
exec "$(dirname "${BASH_SOURCE[0]}")/build-zorin.sh" --build
