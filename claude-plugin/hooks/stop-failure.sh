#!/bin/bash
# stop-failure.sh — StopFailure hook (#197).
#
# Fires when a turn ends due to an API error. Broadcasts the error
# to bpmux so the team sees it instead of silence.
#
# DEBOUNCED: only sends one message per 60-second window per role.
# Without debounce, rate-limit retries flood the chat with repeated
# "hit api_error" messages — noise, not signal.

set -euo pipefail

BPMUX_PORT="${BPMUX_PORT:-4117}"
BPMUX_ROLE="${BPMUX_ROLE:-unknown}"
BPMUX_API="http://127.0.0.1:${BPMUX_PORT}/api/messages"

# Debounce: check if we sent a failure signal in the last 60 seconds
DEBOUNCE_FILE="/tmp/bpmux-stop-failure-${BPMUX_ROLE}"
DEBOUNCE_SECONDS=60

if [ -f "$DEBOUNCE_FILE" ]; then
  LAST_SEND=$(cat "$DEBOUNCE_FILE" 2>/dev/null || echo "0")
  NOW=$(date +%s)
  ELAPSED=$((NOW - LAST_SEND))
  if [ "$ELAPSED" -lt "$DEBOUNCE_SECONDS" ]; then
    # Already sent recently — suppress this one
    exit 0
  fi
fi

# Record the send time
date +%s > "$DEBOUNCE_FILE"

# Read stdin JSON to extract error details (best-effort)
INPUT=$(cat 2>/dev/null || echo "{}")
ERROR_TYPE=$(echo "$INPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('matched_hook','api_error'))" 2>/dev/null || echo "api_error")

MESSAGE_TEXT="${BPMUX_ROLE} hit ${ERROR_TYPE} — temporarily blocked"

curl -s -X POST "$BPMUX_API" \
  -H "Content-Type: application/json" \
  --max-time 3 \
  -d "{\"from\":\"${BPMUX_ROLE}:claude-code\",\"text\":\"${MESSAGE_TEXT}\"}" \
  >/dev/null 2>&1 || true

exit 0
