#!/bin/bash
# send-claude-response.sh — Stop hook
# Appends Claude's response to the bpmux JSONL log so the full
# conversation is captured. Fails silently if bpmux isn't running.

MESSAGES_FILE="$HOME/.bpmux/sessions/current/messages-current.jsonl"

# Do nothing if no active session
if [ ! -f "$MESSAGES_FILE" ]; then
  exit 0
fi

# Read hook input from stdin
INPUT=$(cat 2>/dev/null)

if [ -z "$INPUT" ]; then
  exit 0
fi

# Extract Claude's response text from the hook JSON
# Claude Code passes { "response": "..." } on stdin for Stop hooks
RESPONSE=""
if command -v jq >/dev/null 2>&1; then
  RESPONSE=$(echo "$INPUT" | jq -r '.response // empty' 2>/dev/null)
fi

if [ -z "$RESPONSE" ]; then
  exit 0
fi

# Build the JSONL message
TS=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
# Use jq to safely escape the response text
if command -v jq >/dev/null 2>&1; then
  MESSAGE=$(jq -nc --arg from "ryan:claude-code" --arg text "$RESPONSE" --arg ts "$TS" \
    '{"from":$from,"text":$text,"ts":$ts}')
  echo "$MESSAGE" >> "$MESSAGES_FILE"
fi

exit 0
