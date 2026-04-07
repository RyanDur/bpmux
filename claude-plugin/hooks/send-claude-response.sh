#!/bin/bash
# send-claude-response.sh — Stop hook
# Checks for new bpmux messages. If any, continues with a signal.
# Lean: only count + latest sender, not full content.

BPMUX_PORT="${BPMUX_PORT:-4117}"
BPMUX_API="http://127.0.0.1:${BPMUX_PORT}/api/messages"
LAST_SEEN_FILE="/tmp/bpmux-last-seen-$PPID"

if ! command -v jq >/dev/null 2>&1; then
  exit 0
fi

MESSAGES=$(curl -s "$BPMUX_API" 2>/dev/null)
if [ -z "$MESSAGES" ] || [ "$MESSAGES" = "null" ]; then
  exit 0
fi

LAST_SEEN=""
if [ -f "$LAST_SEEN_FILE" ]; then
  LAST_SEEN=$(cat "$LAST_SEEN_FILE" 2>/dev/null)
fi

# First run — mark position, don't inject
if [ -z "$LAST_SEEN" ]; then
  echo "$MESSAGES" | jq -r '.[-1].ts // empty' > "$LAST_SEEN_FILE" 2>/dev/null
  exit 0
fi

# Count new messages from others
NEW_COUNT=$(echo "$MESSAGES" | jq --arg last "$LAST_SEEN" '
  [.[] |
    select(.ts > $last) |
    select((.message_type // "chat") == "chat") |
    select(.from | test("ryan:claude-code|ryan:mcp-client") | not)
  ] | length
' 2>/dev/null)

# Update last seen
echo "$MESSAGES" | jq -r '.[-1].ts // empty' > "$LAST_SEEN_FILE" 2>/dev/null

if [ "$NEW_COUNT" = "0" ] || [ -z "$NEW_COUNT" ]; then
  exit 0
fi

# Signal only — count, not content
jq -nc --arg count "$NEW_COUNT" '{
  "continue": true,
  "systemMessage": ($count + " new bpmux messages arrived.")
}'

exit 0
