#!/bin/bash
# set-status-help.sh — PermissionRequest hook
# Sets agent status to "help" (red dot) when waiting on user permission.

BPMUX_PORT="${BPMUX_PORT:-4117}"
BPMUX_ROLE="${BPMUX_ROLE:-unknown}"
BPMUX_API="http://127.0.0.1:${BPMUX_PORT}/api/messages"

curl -s -X POST "$BPMUX_API" \
  -H "Content-Type: application/json" \
  -d "{\"from\":\"${BPMUX_ROLE}:claude-code\",\"text\":\"help\",\"message_type\":\"activity\"}" \
  >/dev/null 2>&1

exit 0
