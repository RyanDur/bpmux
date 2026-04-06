#!/bin/bash
# set-status-busy.sh — PreToolCall hook
# Sets agent status to "busy" via the REST API whenever a tool call starts.

BPMUX_PORT="${BPMUX_PORT:-4117}"
BPMUX_ROLE="${BPMUX_ROLE:-unknown}"
BPMUX_API="http://127.0.0.1:${BPMUX_PORT}/api/messages"

curl -s -X POST "$BPMUX_API" \
  -H "Content-Type: application/json" \
  -d "{\"from\":\"${BPMUX_ROLE}:claude-code\",\"text\":\"busy\",\"message_type\":\"activity\"}" \
  >/dev/null 2>&1

exit 0
