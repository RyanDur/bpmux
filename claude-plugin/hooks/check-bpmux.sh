#!/bin/bash
# check-bpmux.sh — UserPromptSubmit hook
# Reads the last 10 messages from the bpmux JSONL log and outputs them
# so Claude sees recent session context before processing the user's prompt.
# Exits silently if bpmux isn't running or the log doesn't exist.

MESSAGES_FILE="$HOME/.bpmux/sessions/current/messages-current.jsonl"

if [ ! -f "$MESSAGES_FILE" ]; then
  exit 0
fi

LINES=$(tail -n 10 "$MESSAGES_FILE" 2>/dev/null)

if [ -z "$LINES" ]; then
  exit 0
fi

echo "--- Recent bpmux messages ---"
while IFS= read -r line; do
  sender=$(echo "$line" | grep -o '"sender":"[^"]*"' | head -1 | cut -d'"' -f4)
  text=$(echo "$line" | grep -o '"text":"[^"]*"' | head -1 | cut -d'"' -f4)
  if [ -n "$sender" ] && [ -n "$text" ]; then
    echo "[$sender] $text"
  fi
done <<< "$LINES"
echo "--- End bpmux messages ---"

exit 0
