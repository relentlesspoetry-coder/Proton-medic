#!/usr/bin/env bash
# Run the complete native build gate.
set -Eeuo pipefail
exec "$(dirname "${BASH_SOURCE[0]}")/build-zorin.sh" --build
