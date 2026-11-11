#!/usr/bin/env bash
# Boots only the existing isolated native harness; never uses the desktop socket.
set -euo pipefail
root=$(cd "$(dirname "$0")/../../../.." && pwd)
native=${1:-$HOME/src/umbriel-working}
meson compile -C "$native/build-debug" umbriel harness-clients
export ZK_BOUNDARY_ROOT="$root"
CHECK_DIR="$root/desktop/umbriel/tests/native/checks" \
  bash "$native/tests/harness/check.sh" "$native/build-debug/umbriel"
