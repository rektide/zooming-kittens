#!/usr/bin/env bash
# journal.sh - Stream logs for kitty-zoom service

set -euo pipefail

# Service name (matches --unit in run.sh)
UNIT_NAME="kitty-zoom.service"

# Try to get PID of running service
PID=$(systemctl --user show --property=MainPID --value "$UNIT_NAME" 2>/dev/null || echo "")

if [ -n "$PID" ]; then
    echo "Following logs for $UNIT_NAME (PID: $PID)..."
    echo "Press Ctrl+C to stop following"
    echo ""
    journalctl --user \
        --identifier="kitty-zoom" \
        -f
else
    echo "No running service found"
    echo ""
    echo "Looking for recent logs..."
    echo ""
    journalctl --user \
        --identifier="kitty-zoom" \
        --no-pager \
        -n 50
fi

# Explanation:
# 
# How journalctl finds the unit:
# 
# 1. When using systemd-run --unit=kitty-zoom, systemd creates a transient service with that exact name
# 2. journalctl can match by unit name (--user -u kitty-zoom.service)
# 3. journalctl can also match by syslog identifier (--identifier=kitty-zoom)
# 4. Using --identifier is more reliable for transient units
# 5. Binary name "kitty-focus-tracker" defaults to syslog identifier "kitty-focus-tracker"
# 6. For consistency, both scripts use "kitty-zoom" identifier
