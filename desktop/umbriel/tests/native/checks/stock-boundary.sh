#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$UMBRIEL_HARNESS_LIB")/title_bar.sh"
cat >> "$UMBRIEL_CONFIG" <<'EOF'
[animation]
enabled = false
[input.focus]
follows_mouse = false
EOF
"$UMBRIEL" msg config-reload > /dev/null
spawn() {
  "$UMBRIEL_FRACTIONAL_CLIENT" "$1" 480 300 > "$UMBRIEL_RUNTIME_DIR/$1.log" 2>&1 &
  title_bar_wait_field "$1" title "$1"
  "$UMBRIEL" settle
}
spawn zk-boundary-anchor
anchor=$(title_bar_window zk-boundary-anchor | jq -r .id)
spawn zk-boundary-upper
spawn zk-boundary-lower
"$UMBRIEL" msg window-consume-left > /dev/null
"$UMBRIEL" settle
"$UMBRIEL" msg window-focus "$anchor" > /dev/null
"$UMBRIEL" settle
export ZK_HEADLESS_BOUNDARY=1
cargo test --locked --manifest-path "$ZK_BOUNDARY_ROOT/Cargo.toml" \
  -p zk-umbriel --test native_stock -- --ignored
