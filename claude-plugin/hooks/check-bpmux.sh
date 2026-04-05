#!/bin/bash
# check-bpmux.sh — UserPromptSubmit hook
# Lightweight context injection: count + latest message only.
# Agents use read_messages tool to pull full context when needed.

MESSAGES_FILE="$HOME/.bpmux/sessions/current/messages-current.jsonl"

if [ ! -f "$MESSAGES_FILE" ]; then
  exit 0
fi

if ! command -v jq >/dev/null 2>&1; then
  exit 0
fi

# Count non-system chat messages
COUNT=$(tail -n 20 "$MESSAGES_FILE" 2>/dev/null | jq -r 'select((.message_type // "chat") == "chat") | .from' 2>/dev/null | wc -l | tr -d ' ')

if [ "$COUNT" = "0" ]; then
  exit 0
fi

# Get only the latest chat message
LATEST=$(tail -n 20 "$MESSAGES_FILE" 2>/dev/null | jq -r 'select((.message_type // "chat") == "chat")' 2>/dev/null | tail -n 1)
FROM=$(echo "$LATEST" | jq -r '.from // empty' 2>/dev/null)
TEXT=$(echo "$LATEST" | jq -r '.text // empty' 2>/dev/null)

if [ -z "$FROM" ] || [ -z "$TEXT" ]; then
  exit 0
fi

# Truncate preview to 100 chars
PREVIEW=$(echo "$TEXT" | cut -c1-100)

echo "bpmux: $COUNT recent messages. Latest from $FROM: $PREVIEW"
echo "Use read_messages to see full conversation."

exit 0
