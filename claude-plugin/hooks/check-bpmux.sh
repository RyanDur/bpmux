#!/bin/bash
# check-bpmux.sh — UserPromptSubmit hook
# Reads recent bpmux messages and injects only relevant ones into context.
# Filters: skip system messages, skip own messages, optionally filter by @mention.
# Exits silently if bpmux isn't running or the log doesn't exist.

MESSAGES_FILE="$HOME/.bpmux/sessions/current/messages-current.jsonl"
# Self-filtering: future improvement — skip own messages
# For now, only system messages are filtered
MY_NAME=""

if [ ! -f "$MESSAGES_FILE" ]; then
  exit 0
fi

LINES=$(tail -n 10 "$MESSAGES_FILE" 2>/dev/null)

if [ -z "$LINES" ]; then
  exit 0
fi

if ! command -v jq >/dev/null 2>&1; then
  exit 0
fi

FILTERED=""
while IFS= read -r line; do
  from=$(echo "$line" | jq -r '.from // empty' 2>/dev/null)
  text=$(echo "$line" | jq -r '.text // empty' 2>/dev/null)
  msg_type=$(echo "$line" | jq -r '.message_type // "chat"' 2>/dev/null)

  # Skip system messages
  [ "$msg_type" = "system" ] && continue

  # Skip empty
  [ -z "$from" ] || [ -z "$text" ] && continue

  # Skip own messages if BPMUX_NAME is set
  if [ -n "$MY_NAME" ]; then
    case "$from" in
      *"$MY_NAME"*) continue ;;
    esac
  fi

  FILTERED="${FILTERED}[$from] $text\n"
done <<< "$LINES"

if [ -z "$FILTERED" ]; then
  exit 0
fi

echo "--- Recent bpmux messages ---"
printf "%b" "$FILTERED"
echo "--- End bpmux messages ---"

exit 0
