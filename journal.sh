#!/usr/bin/env bash
# journal.sh - Stream logs for the zk transient service

set -euo pipefail

# Get service name from ZOOMING_APPNAME env var (defaults to kitty)
SERVICE_NAME="${ZOOMING_APPNAME:-kitty}.service"

# Try to get PID of running service
PID=$(systemctl --user show --property=MainPID --value "$SERVICE_NAME" 2>/dev/null || echo "")

if [ -n "$PID" ] && [ "$PID" != "0" ]; then
    echo "Following logs for $SERVICE_NAME (PID: $PID)..."
    echo "Press Ctrl+C to stop following"
    echo ""
    journalctl --user \
        --unit="$SERVICE_NAME" \
        -f \
        "$@"
else
    echo "No running service found"
    echo ""
    echo "Looking for recent logs..."
    echo ""
    journalctl --user \
        --unit="$SERVICE_NAME" \
        --no-pager \
        -n 50 \
        "$@"
fi

# Match the unit selected by run.sh, independently of the binary's journal identifier.
