#!/bin/bash
# send-user-message.sh — UserPromptSubmit hook
# Posts the user's prompt to the bpmux REST endpoint so the message
# goes through the broadcast channel (TUI + WebSocket clients see it).
# Fails silently if bpmux isn't running.

BPMUX_API="http://127.0.0.1:4117/api/messages"

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

# POST to bpmux REST endpoint
if command -v jq >/dev/null 2>&1; then
  BODY=$(jq -nc --arg from "ryan:claude-code-user" --arg text "$PROMPT" \
    '{"from":$from,"text":$text}')
  curl -s -X POST "$BPMUX_API" \
    -H "Content-Type: application/json" \
    -d "$BODY" >/dev/null 2>&1
fi

exit 0
