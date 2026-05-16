#!/usr/bin/env bash
# bpmux-broadcast-vocabulary-catch hook
# Catches self-imposed-pause anti-pattern at substrate layer (#493).
#
# Triggers on: mcp__bpmux-bridge__send_message
# Banned phrases (signal self-imposed pause):
#   "standing by", "monitoring", "active-trigger-watch", "presence-mode",
#   "watching for", "hold position", "holds", "passive watch", "passive-watch"
# Allowed when accompanied by a NAMED stop-cause:
#   "queue-empty", "blocker", "owner-signal", "red-CI", "red ci",
#   "next-pull", "substrate-bound", "legitimate-deferral",
#   "named-active-trigger", "named-empty"
#
# When banned phrase appears WITHOUT a named cause, the hook returns
# permissionDecision="ask" so Claude must justify or rewrite before sending.

if ! command -v jq &>/dev/null; then
  exit 0
fi

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

if [ "$TOOL_NAME" != "mcp__bpmux-bridge__send_message" ]; then
  exit 0
fi

TEXT=$(echo "$INPUT" | jq -r '.tool_input.text // empty')
if [ -z "$TEXT" ]; then
  exit 0
fi

LOWER=$(echo "$TEXT" | tr '[:upper:]' '[:lower:]')

# Strip quoted spans before banned-phrase match so narrative-quoted
# self-correction broadcasts (e.g., PM saying `used "standing by"
# earlier — apologies`) don't trip the hook. #1372: false-positive
# rate on correction-class broadcasts must be 0. Named-cause check
# stays against the FULL text so "blocker" inside quotes still
# satisfies the named-cause gate.
STRIPPED=$(echo "$LOWER" | sed -e "s/'[^']*'//g" -e 's/"[^"]*"//g' -e 's/`[^`]*`//g')

BANNED_PATTERN='standing by|monitoring|active-trigger-watch|presence-mode|watching for|hold position|passive watch|passive-watch'

if ! echo "$STRIPPED" | grep -qE "$BANNED_PATTERN"; then
  exit 0
fi

NAMED_CAUSE_PATTERN='queue-empty|blocker|owner-signal|red-ci|red ci|next-pull|substrate-bound|legitimate-deferral|named-active-trigger|named-empty|awaiting-owner|ci-red|test-failure'

if echo "$LOWER" | grep -qE "$NAMED_CAUSE_PATTERN"; then
  exit 0
fi

MATCHED=$(echo "$STRIPPED" | grep -oE "$BANNED_PATTERN" | head -1)

REASON="self-imposed-pause vocabulary detected (\"$MATCHED\") without named stop-cause. Per #493 substrate-fix: broadcasts that signal pause/standby require a NAMED cause (queue-empty, blocker, owner-signal, red-CI, next-pull, substrate-bound, legitimate-deferral, named-active-trigger). Either: (1) rewrite to explicit active work + commit-beat, or (2) name the stop-cause explicitly. See feedback_silent_while_working_reads_as_stopped."

jq -n \
  --arg reason "$REASON" \
  '{
    "hookSpecificOutput": {
      "hookEventName": "PreToolUse",
      "permissionDecision": "ask",
      "permissionDecisionReason": $reason
    }
  }'
