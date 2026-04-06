#!/bin/bash
# set-status-available.sh — Stop hook
# Sets agent status back to "available" when the agent's turn ends.

BPMUX_PORT="${BPMUX_PORT:-4117}"
BPMUX_ROLE="${BPMUX_ROLE:-unknown}"
BPMUX_API="http://127.0.0.1:${BPMUX_PORT}/api/messages"

curl -s -X POST "$BPMUX_API" \
  -H "Content-Type: application/json" \
  -d "{\"from\":\"${BPMUX_ROLE}:claude-code\",\"text\":\"available\",\"message_type\":\"activity\"}" \
  >/dev/null 2>&1

exit 0
