#!/usr/bin/env bash
# Ordinary stock executable acceptance; only Umbriel's contained harness socket.
set -euo pipefail
: "${ZK_TEST_BIN:?absolute path to built zk}"
: "${UMBRIEL_RUNTIME_DIR:?run through the native headless harness}"
: "${UMBRIEL_REPO:?native harness checkout}"
root=$(cd "$(dirname "$0")/../../../.." && pwd)
mkdir -p "$root/.test-agent/longcat-feedback-e"
evidence=$(mktemp -d "$root/.test-agent/longcat-feedback-e/native.XXXXXXXX")
preserve() {
  local file
  for file in "$UMBRIEL_RUNTIME_DIR"/*.jsonl "$UMBRIEL_RUNTIME_DIR"/*.stderr \
    "$UMBRIEL_RUNTIME_DIR"/*.log "$UMBRIEL_CONFIG"; do
    [[ ! -f $file ]] || cp "$file" "$evidence/"
  done
  echo "native evidence: $evidence"
}
trap preserve EXIT
# Reuse C's real xdg constraint/delayed-commit client, not another client model.
cc -std=c11 -Wall -Wextra -Werror $(pkg-config --cflags wayland-client) \
  -I "$(dirname "$UMBRIEL")" "$root/desktop/umbriel/tests/native/edge-client.c" \
  "$(dirname "$UMBRIEL")/xdg-shell-client-protocol.c" $(pkg-config --libs wayland-client) \
  -o "$UMBRIEL_RUNTIME_DIR/edge-client"
export ZK_EDGE_CLIENT="$UMBRIEL_RUNTIME_DIR/edge-client"
python3 "$root/app/zk/tests/headless/stock_feedback.py"
