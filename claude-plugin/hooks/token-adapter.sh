#!/bin/bash
# token-adapter.sh — Stop hook adapter for #179 token observability.
#
# Reads Claude Code's transcript (path passed in the Stop hook JSON on
# stdin), finds the last assistant entry's usage data, and appends a
# row to ~/.bpmux/tokens/log.jsonl in the common schema defined at
# docs/token-log-schema.md.
#
# **Dedup**: we only emit a row if the last-seen assistant uuid has
# changed since the previous run, so multiple Stop fires (e.g., from
# hook replays) don't double-count the same turn.
#
# **Scope limitation (v1)**: captures ONLY the last assistant entry per
# Stop fire, not every LLM call in the turn. A turn with multiple
# tool-loop round-trips will only have its final LLM call recorded.
# This is the headline cost; finer granularity can land in a follow-up
# adapter revision if/when the `/tokens` view needs it. The schema is
# designed so upgrading the adapter later doesn't require changing
# bpmux code — the log format stays the same.

set -e

if ! command -v jq >/dev/null 2>&1; then
  exit 0
fi

BPMUX_ROLE="${BPMUX_ROLE:-unknown}"
LOG_DIR="${BPMUX_TOKENS_DIR:-$HOME/.bpmux/tokens}"
LOG_FILE="$LOG_DIR/log.jsonl"

# Read the Stop hook JSON from stdin
HOOK_JSON=$(cat)
TRANSCRIPT_PATH=$(echo "$HOOK_JSON" | jq -r '.transcript_path // empty')
SESSION_ID=$(echo "$HOOK_JSON" | jq -r '.session_id // empty')

if [ -z "$TRANSCRIPT_PATH" ] || [ ! -f "$TRANSCRIPT_PATH" ]; then
  exit 0
fi

# Extract the LAST assistant entry with usage data.
# jq streams forward and we take the last match — this avoids relying
# on `tac` / `tail -r` which differ across GNU and BSD coreutils.
LAST_ENTRY=$(jq -c 'select(.type == "assistant" and .message.usage != null)' "$TRANSCRIPT_PATH" 2>/dev/null | tail -1)

if [ -z "$LAST_ENTRY" ]; then
  exit 0
fi

LAST_UUID=$(echo "$LAST_ENTRY" | jq -r '.uuid // empty')

# Dedup: skip if we've already written this uuid for this session
mkdir -p "$LOG_DIR"
STATE_FILE="$LOG_DIR/.last-uuid-${SESSION_ID}"
if [ -f "$STATE_FILE" ] && [ "$(cat "$STATE_FILE" 2>/dev/null)" = "$LAST_UUID" ]; then
  exit 0
fi

MODEL=$(echo "$LAST_ENTRY" | jq -r '.message.model // "unknown"')
INPUT=$(echo "$LAST_ENTRY" | jq -r '.message.usage.input_tokens // 0')
CACHE_CREATE=$(echo "$LAST_ENTRY" | jq -r '.message.usage.cache_creation_input_tokens // 0')
CACHE_READ=$(echo "$LAST_ENTRY" | jq -r '.message.usage.cache_read_input_tokens // 0')
OUTPUT=$(echo "$LAST_ENTRY" | jq -r '.message.usage.output_tokens // 0')
TS=$(echo "$LAST_ENTRY" | jq -r '.timestamp // empty')
if [ -z "$TS" ]; then
  TS=$(date -u +"%Y-%m-%dT%H:%M:%S+00:00")
fi

# Tool call count: number of tool_use blocks in the message content
TOOL_CALLS=$(echo "$LAST_ENTRY" | jq '[.message.content[]? | select(.type == "tool_use")] | length')

# Write row atomically-ish (one jq output, one append)
jq -nc \
  --arg ts "$TS" \
  --arg role "$BPMUX_ROLE" \
  --arg harness "claude-code" \
  --arg model "$MODEL" \
  --argjson input "$INPUT" \
  --argjson cc "$CACHE_CREATE" \
  --argjson cr "$CACHE_READ" \
  --argjson output "$OUTPUT" \
  --argjson tools "$TOOL_CALLS" \
  --arg session "$SESSION_ID" \
  '{
    ts: $ts,
    role: $role,
    harness: $harness,
    model: $model,
    input_tokens: $input,
    cache_creation_input_tokens: $cc,
    cache_read_input_tokens: $cr,
    output_tokens: $output,
    tool_call_count: $tools,
    session_id: $session
  }' >> "$LOG_FILE"

# Mark this uuid as processed
echo "$LAST_UUID" > "$STATE_FILE"

exit 0
