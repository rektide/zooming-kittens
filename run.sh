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

# Get service name from ZOOMING_APPNAME env var (defaults to zooming-kittens)
SERVICE_NAME="${ZOOMING_APPNAME:-zooming-kittens}"

# Start via systemd-run
# Using --unit makes unit name predictable
echo "Starting $SERVICE_NAME via systemd-run..."
systemd-run --user \
    --quiet \
    --unit="$SERVICE_NAME" \
    --setenv=RUST_BACKTRACE=full \
    --setenv=ZOOMING_APPNAME="$SERVICE_NAME" \
    "$PROJECT_DIR/target/release/kitty-focus-tracker" --verbose

echo ""
echo "$SERVICE_NAME is running"
echo "Unit name: $SERVICE_NAME.service"
echo ""
echo "To view logs:"
echo "  ./journal.sh"
echo ""
echo "Or use directly:"
echo "  journalctl --user -u $SERVICE_NAME.service -f"
echo ""
echo "To stop:"
echo "  systemctl --user stop $SERVICE_NAME.service"
