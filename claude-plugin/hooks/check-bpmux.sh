#!/bin/bash
# check-bpmux.sh — UserPromptSubmit hook
# Only outputs when NEW messages exist since last check.
# Tracks last-seen line count per session.

DATA_BASE="$HOME/.bpmux/data"
if [ ! -d "$DATA_BASE" ]; then
  DATA_BASE="$HOME/.bpmux/sessions"
fi
DATA_BASE="${BPMUX_DATA_BASE:-$DATA_BASE}"
MESSAGES_FILE="$DATA_BASE/current/messages-current.jsonl"

if [ ! -f "$MESSAGES_FILE" ]; then
  exit 0
fi

# Track last seen line count
TOTAL_LINES=$(wc -l < "$MESSAGES_FILE" 2>/dev/null | tr -d ' ')
# Use session-specific last-seen file based on messages file path
LAST_SEEN_FILE="/tmp/bpmux-hook-$(echo "$MESSAGES_FILE" | md5sum 2>/dev/null | cut -c1-8 || echo "default")"

LAST_SEEN=0
if [ -f "$LAST_SEEN_FILE" ]; then
  LAST_SEEN=$(cat "$LAST_SEEN_FILE" 2>/dev/null)
fi

# No new lines since last check
if [ "$TOTAL_LINES" -le "$LAST_SEEN" ] 2>/dev/null; then
  exit 0
fi

# Update last seen
echo "$TOTAL_LINES" > "$LAST_SEEN_FILE"

NEW_COUNT=$((TOTAL_LINES - LAST_SEEN))

if ! command -v jq >/dev/null 2>&1; then
  echo "bpmux: $NEW_COUNT new messages."
  exit 0
fi

# Get the latest chat message
LATEST=$(tail -n "$NEW_COUNT" "$MESSAGES_FILE" 2>/dev/null | jq -c 'select((.message_type // "chat") == "chat")' 2>/dev/null | tail -n 1)
FROM=$(echo "$LATEST" | jq -r '.from // empty' 2>/dev/null)
TEXT=$(echo "$LATEST" | jq -r '.text // empty' 2>/dev/null)

if [ -z "$FROM" ] || [ -z "$TEXT" ]; then
  echo "bpmux: $NEW_COUNT new messages."
  exit 0
fi

PREVIEW=$(echo "$TEXT" | cut -c1-100)
echo "bpmux: $NEW_COUNT new. Latest from $FROM: $PREVIEW"

exit 0
