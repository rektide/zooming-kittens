#!/usr/bin/env bash
# Boots only the existing isolated native harness; never uses the desktop socket.
set -euo pipefail
root=$(cd "$(dirname "$0")/../../../.." && pwd)
native=${1:-$HOME/src/umbriel-working}
meson compile -C "$native/build-debug" umbriel harness-clients
mkdir -p "$root/.test-agent/longcat-native"
cat > "$root/.test-agent/longcat-native/README.md" <<'EOF'
# Isolated Longcat native test build

Generated edge-client binary for the committed native C fixture. No desktop
socket or installed service is used. Rebuild with desktop/umbriel/tests/native/check.sh.
EOF
cc -std=c11 -Wall -Wextra -Werror $(pkg-config --cflags wayland-client) \
  -I "$native/build-debug" "$root/desktop/umbriel/tests/native/edge-client.c" \
  "$native/build-debug/xdg-shell-client-protocol.c" $(pkg-config --libs wayland-client) \
  -o "$root/.test-agent/longcat-native/edge-client"
export ZK_BOUNDARY_ROOT="$root"
export ZK_EDGE_CLIENT="$root/.test-agent/longcat-native/edge-client"
CHECK_DIR="$root/desktop/umbriel/tests/native/checks" \
  bash "$native/tests/harness/check.sh" "$native/build-debug/umbriel"
