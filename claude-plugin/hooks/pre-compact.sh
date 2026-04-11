#!/bin/bash
# pre-compact.sh — PreCompact hook (#166).
#
# Fires before Claude Code's context compaction reduces the session
# window. Sends a short "I am compacting, back shortly" broadcast to
# bpmux so the team sees the event as a visible transition instead of
# as silent disappearance. After compaction the agent's session-start
# hook runs, which re-loads memory files and surfaces recent messages,
# so the absence is bounded — but without this ping, teammates can't
# distinguish "compacting" from "dead" and may wait indefinitely (see
# the 2026-04-10 nav/drive failure mode where agents sat silent
# through each other's context events).
#
# The hook calls bpmux's REST POST endpoint (same path the other
# plugin hooks use for status updates). The message is a normal chat
# message — NOT an activity/system marker — because the team needs to
# SEE the compaction in the conversation, not just in a sidebar
# activity indicator.
#
# Timeout: short. Claude Code's hook runner will kill this after the
# timeout in hooks.json, and compaction proceeds either way. Failing
# to send the signal is non-fatal; the agent will just re-appear via
# the session-start hook on the other side with slightly more
# ambiguity than if the signal had landed.

set -euo pipefail

BPMUX_PORT="${BPMUX_PORT:-4117}"
BPMUX_ROLE="${BPMUX_ROLE:-unknown}"
BPMUX_API="http://127.0.0.1:${BPMUX_PORT}/api/messages"

# Build the message body. Short by design — well under the 64 KB ws
# cap, and intentionally laconic because the signal doesn't need to
# carry details; it just needs to mark the boundary. Teammates read
# "compacting, brb" and know to wait for the session-start ping
# rather than assume the agent dropped.
MESSAGE_TEXT="compacting context, back shortly"

curl -s -X POST "$BPMUX_API" \
  -H "Content-Type: application/json" \
  --max-time 3 \
  -d "{\"from\":\"${BPMUX_ROLE}:claude-code\",\"text\":\"${MESSAGE_TEXT}\"}" \
  >/dev/null 2>&1 || true

# Always exit 0. A failed send should not block the compaction —
# compaction is load-bearing for the agent's ability to keep working,
# and a missing signal is a soft cost, not a reason to crash.
exit 0
