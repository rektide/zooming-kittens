#!/usr/bin/env bash
# run.sh - Start kitty-focus-tracker via systemd-run and stream logs

set -euo pipefail

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Build project if not already built
if [ ! -f "$PROJECT_DIR/target/release/kitty-focus-tracker" ]; then
    echo "Building kitty-focus-tracker in release mode..."
    cd "$PROJECT_DIR"
    cargo build --release
fi

# Start via systemd-run
# Using --unit makes unit name predictable
echo "Starting kitty-zoom via systemd-run..."
systemd-run --user \
    --quiet \
    --unit=kitty-zoom \
    --setenv=RUST_BACKTRACE=full \
    "$PROJECT_DIR/target/release/kitty-focus-tracker" --verbose

echo ""
echo "kitty-zoom is running"
echo "Unit name: kitty-zoom.service"
echo ""
echo "To view logs:"
echo "  ./journal.sh"
echo ""
echo "Or use directly:"
echo "  journalctl --user -u kitty-zoom.service -f"
echo ""
echo "To stop:"
echo "  systemctl --user stop kitty-zoom.service"
