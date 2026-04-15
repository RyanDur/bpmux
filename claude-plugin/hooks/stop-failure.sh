#!/bin/bash
# stop-failure.sh — StopFailure hook (#197).
#
# Fires when a turn ends due to an API error (rate_limit,
# server_error, authentication_failed, billing_error).
# Broadcasts the error to bpmux so the team sees it instead
# of silence.
#
# Without this hook, rate limits are invisible to teammates —
# the agent goes quiet and nobody knows why. With it, the team
# sees "dev hit rate_limit" and knows the agent is temporarily
# blocked, not dead.
#
# The matcher in hooks.json filters which error types trigger
# this script. The error type is extracted from stdin JSON
# (hook_event_name field) or from the matched pattern.

set -euo pipefail

BPMUX_PORT="${BPMUX_PORT:-4117}"
BPMUX_ROLE="${BPMUX_ROLE:-unknown}"
BPMUX_API="http://127.0.0.1:${BPMUX_PORT}/api/messages"

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
