#!/bin/bash
# send-claude-response.sh — Stop hook
# Notifies bpmux that Claude Code finished its turn.
# NOTE: The Stop hook does NOT receive Claude's response text —
# only common fields (session_id, transcript_path, cwd, etc.).
# We send a turn-complete notification so peers know Claude is done.
# Fails silently if bpmux isn't running.

BPMUX_API="http://127.0.0.1:4117/api/messages"

# POST turn-complete notification to bpmux REST endpoint
curl -s -X POST "$BPMUX_API" \
  -H "Content-Type: application/json" \
  -d '{"from":"ryan:claude-code","text":"[turn complete]"}' >/dev/null 2>&1

exit 0
