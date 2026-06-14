#!/bin/bash
# session-end.sh — SessionEnd hook
#
# Clean-exit leave for the liveness model (#1721): when a session ends cleanly
# (/exit, /clear, resume, logout), drop this agent's roster row INSTANTLY rather
# than waiting out the keepalive grace. SessionEnd fires only on clean exits — an
# ungraceful kill is covered separately by the keepalive's PID-watch + grace — so
# this is a latency optimization on the clean path, never the sole leave-signal.
#
# Order matters: kill the keepalive FIRST (via the session-id-keyed pidfile the
# SessionStart hook wrote) so no in-flight ping can resurrect the agent after the
# disconnect, THEN fire the clean disconnect.

HOOK_INPUT=$(cat)

session_id=$(printf '%s' "$HOOK_INPUT" \
    | grep -o '"session_id"[[:space:]]*:[[:space:]]*"[^"]*"' \
    | sed 's/.*:[[:space:]]*"//; s/"$//')

if [ -n "$session_id" ]; then
    pidfile="${TMPDIR:-/tmp}/bpmux-keepalive-${session_id}.pid"
    if [ -f "$pidfile" ]; then
        keepalive_pid=$(cat "$pidfile" 2>/dev/null)
        [ -n "$keepalive_pid" ] && kill "$keepalive_pid" 2>/dev/null || true
        rm -f "$pidfile" 2>/dev/null || true
    fi
fi

# Find the bpmux binary (same resolution order as session-start.sh).
BPMUX_BIN=""
for candidate in \
    "${BPMUX_BIN_OVERRIDE:-}" \
    "$HOME/IdeaProjects/bpmux/target/release/bpmux" \
    "$HOME/IdeaProjects/bpmux/target/debug/bpmux" \
    "$(command -v bpmux 2>/dev/null)"
do
    if [ -n "$candidate" ] && [ -x "$candidate" ]; then
        BPMUX_BIN="$candidate"
        break
    fi
done

if [ -n "$BPMUX_BIN" ] && [ -n "${BPMUX_ROLE:-}" ]; then
    "$BPMUX_BIN" disconnect >/dev/null 2>&1 || true
fi

exit 0
