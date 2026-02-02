#!/usr/bin/env bash
# journal.sh - Stream logs for kitty-zoom service

set -euo pipefail

# Get service name from ZOOMING_APPNAME env var (defaults to zooming-kittens)
SERVICE_NAME="${ZOOMING_APPNAME:-zooming-kittens}.service"

# Try to get PID of running service
PID=$(systemctl --user show --property=MainPID --value "$UNIT_NAME" 2>/dev/null || echo "")

if [ -n "$PID" ]; then
    echo "Following logs for $SERVICE_NAME (PID: $PID)..."
    echo "Press Ctrl+C to stop following"
    echo ""
    journalctl --user \
        --identifier="${SERVICE_NAME%.service}" \
        -f
else
    echo "No running service found"
    echo ""
    echo "Looking for recent logs..."
    echo ""
    journalctl --user \
        --identifier="${SERVICE_NAME%.service}" \
        --no-pager \
        -n 50
fi

# Explanation:
# 
# How journalctl finds the unit:
# 
# 1. Service name comes from ZOOMING_APPNAME env var (defaults to "zooming-kittens")
# 2. When using systemd-run --unit=$SERVICE_NAME, systemd creates a transient service with that name
# 3. journalctl can match by unit name (--user -u $SERVICE_NAME)
# 4. journalctl can also match by syslog identifier (--identifier=${SERVICE_NAME%.service})
# 5. Using --identifier is more reliable for transient units
# 6. Binary name "kitty-focus-tracker" defaults to syslog identifier "kitty-focus-tracker"
# 7. For consistency, both scripts use $SERVICE_NAME identifier
