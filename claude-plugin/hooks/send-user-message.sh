#!/bin/bash
# send-user-message.sh — UserPromptSubmit hook
# Appends the user's prompt to the bpmux JSONL log so the full
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

# Extract the user's prompt text from the hook JSON
# Claude Code passes { "prompt": "..." } on stdin for UserPromptSubmit
PROMPT=""
if command -v jq >/dev/null 2>&1; then
  PROMPT=$(echo "$INPUT" | jq -r '.prompt // empty' 2>/dev/null)
fi

if [ -z "$PROMPT" ]; then
  exit 0
fi

# Build the JSONL message
TS=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
# Use jq to safely escape the prompt text
if command -v jq >/dev/null 2>&1; then
  MESSAGE=$(jq -nc --arg from "ryan:claude-code-user" --arg text "$PROMPT" --arg ts "$TS" \
    '{"from":$from,"text":$text,"ts":$ts}')
  echo "$MESSAGE" >> "$MESSAGES_FILE"
fi

exit 0
