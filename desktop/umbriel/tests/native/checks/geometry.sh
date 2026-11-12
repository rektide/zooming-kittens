#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$UMBRIEL_HARNESS_LIB")/title_bar.sh"
base="$UMBRIEL_RUNTIME_DIR/geometry-base.toml"
cp "$UMBRIEL_CONFIG" "$base"
spawn() {
  "$ZK_EDGE_CLIENT" "$1" > "$UMBRIEL_RUNTIME_DIR/$1.log" 2>&1 &
  title_bar_wait_field "$1" title "$1"
  "$UMBRIEL" settle
}
configure() {
  cp "$base" "$UMBRIEL_CONFIG"
  cat >> "$UMBRIEL_CONFIG" <<EOF
[animation]
enabled = false
[input.focus]
follows_mouse = false
[appearance.title_bar]
mode = "on"
height = 28
[layout]
gap = 10
[layout.scrolling]
center_underfull_strip = false
[layout.struts]
top = $2
bottom = $2
left = $2
right = $2
[[window_rule]]
match.title = "^zk-geometry-"
default_floating = false
[[window_rule]]
match.title = "^zk-geometry-1$"
title_bar = "off"
[output."HEADLESS-1"]
workspace_axis = "$([[ $1 == horizontal ]] && echo vertical || echo horizontal)"
EOF
  "$UMBRIEL" msg config-reload > /dev/null
}
run_case() {
  local axis=$1 strut=$2 count=$3 constraint=$4 delay=$5
  configure "$axis" "$strut"
  spawn zk-geometry-anchor
  anchor=$(title_bar_window zk-geometry-anchor | jq -r .id)
  for i in $(seq 0 $((count-1))); do
    if [[ $constraint == fixed && $i == 1 ]]; then
      export MIN_H=100 MAX_H=100
    elif [[ $constraint == maximum && $i == 0 ]]; then
      export MAX_H=180
    elif [[ $constraint == minimum && $i == 1 ]]; then
      export MIN_H=230
    fi
    export COMMIT_DELAY_MS=$delay
    spawn "zk-geometry-$i"
    unset MIN_H MAX_H COMMIT_DELAY_MS
    if ((i)); then
      # Native consume-left is a logical previous-column action on either axis.
      "$UMBRIEL" msg window-consume-left > /dev/null
      "$UMBRIEL" settle
    fi
  done
  "$UMBRIEL" msg window-focus "$anchor" > /dev/null
  "$UMBRIEL" settle
  "$UMBRIEL" windows --json > "$UMBRIEL_RUNTIME_DIR/geometry-$constraint.json"
  export ZK_HEADLESS_GEOMETRY=1 ZK_GEOMETRY_COUNT=$count ZK_GEOMETRY_CONSTRAINT=$constraint
  export ZK_GEOMETRY_AXIS=$axis ZK_GEOMETRY_DELAY=$delay
  export ZK_GEOMETRY_STRUT=$strut
  cargo test --locked --manifest-path "$ZK_BOUNDARY_ROOT/Cargo.toml" \
    -p zk-umbriel --test native_geometry -- --ignored --nocapture
  # Exact owned harness targets only; no production desktop or automatic pruning.
  for i in $(seq 0 $((count-1))); do
    "$UMBRIEL" msg window-kill "$(title_bar_window "zk-geometry-$i" | jq -r .id)" > /dev/null
  done
  "$UMBRIEL" msg window-kill "$anchor" > /dev/null
  "$UMBRIEL" settle
}
run_case horizontal 20 3 none 0
run_case horizontal -80 4 none 0
run_case vertical 20 4 none 0
run_case vertical -80 3 none 0
run_case horizontal 0 3 none 120
run_case horizontal 0 3 fixed 0
run_case horizontal 0 3 maximum 0
run_case horizontal 0 3 minimum 0
