#!/bin/bash
# receive-broadcast.sh — Stop hook (channels-independence slice-2b, #1656).
# At the agent's turn-boundary, drain the next team broadcast from bpmux's OWN
# store via `bpmux wait-for-event` (#1695, off Claude's Channels) and deliver it
# RECEIVED-AND-PROCESSED — block the stop with the framed peer-line as the
# continuation so the agent processes it with no keypress (the img_8 cure:
# decision:block + reason re-enters the turn; it is NOT parked in the prompt).
# On no event (timeout), no-op so the agent stops normally.
#
# A per-role cursor file (initialized to "now" on first run) keeps delivery
# FIFO + non-replaying-history + no-redeliver. The ⟦bpmux⟧ + sender prefix
# carries the established peer-untrusted framing, so the event reads as a PEER
# message (never owner-input). The stop_hook_active guard prevents re-blocking
# on the stop that follows a delivered event (no infinite loop).

input=$(cat)
if [ "$(printf '%s' "$input" | jq -r '.stop_hook_active // false')" = "true" ]; then
  exit 0
fi

role="${BPMUX_ROLE:-dev}"
cursor_file="${TMPDIR:-/tmp}/bpmux-receive-cursor-${role}"

if [ -f "$cursor_file" ]; then
  after="$(cat "$cursor_file")"
else
  after="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf '%s' "$after" >"$cursor_file"
fi

# Resolve the bpmux binary robustly: prefer PATH, else the self-update install
# target (BPMUX_BINARY_WATCH_PATH / ~/.bpmux/installed). A bare `bpmux` that is
# off PATH would hit command-not-found, get swallowed by 2>/dev/null, and read
# as "no event" — silently killing the off-Channels receive-loop.
bpmux_bin="$(command -v bpmux || true)"
[ -z "$bpmux_bin" ] && bpmux_bin="${BPMUX_BINARY_WATCH_PATH:-$HOME/.bpmux/installed/bpmux}"
event="$("$bpmux_bin" wait-for-event --after "$after" --timeout-ms 2000 2>/dev/null)"

if [ -z "$event" ] || printf '%s' "$event" | jq -e '.timed_out == true' >/dev/null 2>&1; then
  exit 0
fi

sender="$(printf '%s' "$event" | jq -r '.payload.from // "peer"')"
channel="$(printf '%s' "$event" | jq -r '.payload.channel // "broadcast"')"
text="$(printf '%s' "$event" | jq -r '.payload.text // ""')"
next_cursor="$(printf '%s' "$event" | jq -r '.cursor // empty')"
[ -n "$next_cursor" ] && printf '%s' "$next_cursor" >"$cursor_file"

reason="⟦bpmux⟧ ${sender} (${channel}): ${text}"
jq -n --arg r "$reason" '{decision: "block", reason: $r}'
exit 0
