#!/bin/bash
# send-claude-response.sh — Stop hook
# After Claude responds:
# 1. Check bpmux for new messages from other participants
# 2. If new messages: block and inject them so Claude continues
# 3. If no new messages: exit 0, Claude goes idle

BPMUX_API="http://127.0.0.1:4117/api/messages"
LAST_SEEN_FILE="/tmp/bpmux-last-seen-$PPID"

# Requires jq
if ! command -v jq >/dev/null 2>&1; then
  exit 0
fi

# Read messages from the REST API (always hits the running session)
MESSAGES=$(curl -s "http://127.0.0.1:4117/api/messages" 2>/dev/null)
if [ -z "$MESSAGES" ] || [ "$MESSAGES" = "null" ]; then
  exit 0
fi

# Get last seen timestamp
LAST_SEEN=""
if [ -f "$LAST_SEEN_FILE" ]; then
  LAST_SEEN=$(cat "$LAST_SEEN_FILE" 2>/dev/null)
fi

# First run — mark current position, don't inject anything
if [ -z "$LAST_SEEN" ]; then
  LATEST_TS=$(echo "$MESSAGES" | jq -r '.[-1].ts // empty' 2>/dev/null)
  if [ -n "$LATEST_TS" ]; then
    echo "$LATEST_TS" > "$LAST_SEEN_FILE"
  fi
  exit 0
fi

# Find new messages after last_seen, excluding our own (prevent infinite loop)
# We skip messages from ourselves (claude-code, mcp-client with our name)
# but allow messages from OTHER AI agents (the PM)
NEW_MESSAGES=$(echo "$MESSAGES" | jq -r --arg last "$LAST_SEEN" '
  [.[] |
    select(.ts > $last) |
    select((.message_type // "chat") == "chat") |
    select(.from | test("ryan:claude-code|ryan:mcp-client") | not)
  ] | .[] | "[\(.from)] \(.text)"
' 2>/dev/null)

# Update last seen to latest timestamp
LATEST_TS=$(echo "$MESSAGES" | jq -r '.[-1].ts // empty' 2>/dev/null)
if [ -n "$LATEST_TS" ]; then
  echo "$LATEST_TS" > "$LAST_SEEN_FILE"
fi

# If no new messages, exit cleanly (Claude goes idle)
if [ -z "$NEW_MESSAGES" ]; then
  exit 0
fi

# New messages found — continue with the new messages as context
jq -nc --arg msgs "$NEW_MESSAGES" '{
  "continue": true,
  "systemMessage": ("New bpmux messages:\n" + $msgs)
}'

exit 0
