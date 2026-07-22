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

# #279 — auto-save a bookmark BEFORE the broadcast so post-compaction
# load_bookmark works. Two paths:
#   1. Bookmark file already exists (agent ran save_bookmark earlier
#      this session): touch last_updated to mark "preserved through
#      this PreCompact"; agent's structured state is unchanged.
#   2. No prior bookmark: write a minimal stub with timestamp + note
#      "auto-saved at PreCompact" + role-correct path. Post-compaction
#      load_bookmark returns this stub instead of an empty Bookmark,
#      signaling "compaction happened; reconstruct from chat + lessons".
# In both cases the existing chat broadcast still fires (additive
# wiring per falsifier-(f)).
DATA_BASE="$HOME/.bpmux/data"
if [ ! -d "$DATA_BASE" ]; then
  DATA_BASE="$HOME/.bpmux/sessions"
fi
DATA_BASE="${BPMUX_DATA_BASE:-$DATA_BASE}"
DATA_DIR="$DATA_BASE"
if [ -L "$DATA_BASE/current" ] || [ -d "$DATA_BASE/current" ]; then
  DATA_DIR=$(cd "$DATA_BASE/current" 2>/dev/null && pwd -P || echo "$DATA_BASE")
fi
SAFE_ROLE=$(printf '%s' "$BPMUX_ROLE" | sed 's/[^A-Za-z0-9_-]//g')
if [ -z "$SAFE_ROLE" ]; then
  SAFE_ROLE="unknown"
fi
BOOKMARK_PATH="$DATA_DIR/bookmark-${SAFE_ROLE}.json"
TS=$(date -u +%Y-%m-%dT%H:%M:%SZ)

mkdir -p "$DATA_DIR" 2>/dev/null || true

if [ -f "$BOOKMARK_PATH" ]; then
  # Existing bookmark — update last_updated only, preserve everything
  # else. Use python3 so JSON edits don't corrupt the file under a
  # malformed sed. python3 is in plugin-runtime per token-adapter.sh.
  python3 - <<EOF || true
import json, sys
path = "$BOOKMARK_PATH"
ts = "$TS"
try:
    with open(path) as f:
        data = json.load(f)
except Exception:
    data = {}
if not isinstance(data, dict):
    data = {}
data["last_updated"] = ts
with open(path, "w") as f:
    json.dump(data, f, indent=2)
EOF
else
  # No prior bookmark — write stub. Schema matches #168's Bookmark
  # struct (current_task / decisions / open_questions / next_step /
  # notes / last_updated). All-empty except note + timestamp so the
  # schema stays #168-compatible for load_bookmark.
  cat > "$BOOKMARK_PATH" <<EOF || true
{
  "current_task": "",
  "decisions": [],
  "open_questions": [],
  "next_step": "",
  "notes": "auto-saved at PreCompact",
  "last_updated": "$TS"
}
EOF
fi

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
