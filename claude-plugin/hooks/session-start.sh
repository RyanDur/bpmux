#!/bin/bash
# session-start.sh — SessionStart hook
#
# 1. Reminds Claude that bpmux is connected (bridge identity + status rules).
# 2. Surfaces the agent's own per-role lessons from prior sessions so bpmux's
#    lesson index becomes the authoritative auto-loaded memory path (#157).
#    Closes the two-store duplication with Claude Code's auto-memory.
# 3. Spawns the liveness keepalive so an idle agent stays present (#1721).
#
# The recall call reads directly from ~/.bpmux/sessions/current/lessons.json
# (via `bpmux recall`), so it works even if the bpmux TUI isn't running yet.
# Role comes from BPMUX_ROLE, which each workspace's .claude/settings.local.json
# already sets. Missing role → recall silently emits a "no prior lessons"
# line, which is fine.

# The SessionStart payload arrives as JSON on stdin; capture it before writing
# any stdout so the keepalive can read transcript_path to find the agent process.
HOOK_INPUT=$(cat)

echo "bpmux is connected. Call set_name first with your role (e.g. 'dev', 'pm', 'designer') so the team knows who joined. Status is managed automatically by hooks — do not call set_status manually. Messages are pushed to you via the bridge — do not poll read_messages."

# Find the bpmux binary. Prefer a release build, fall back to debug, then
# hope it's on PATH. Silent on failure because session start must not block.
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

# Register this agent's routing identity (channels-independence #1712): derive a
# stable session-id from the workspace directory (held there in a lightweight
# .bpmux-session-id marker, reclaimed across restart-in-the-same-dir) and register
# it server-side, so team-messages route by the durable id and the receive-cursor
# drains from a server-tracked position instead of a stale /tmp file. Idempotent —
# reclaims the same id each start. Graceful: silent no-op if the server is down or
# the binary predates the verb. Non-Claude agents self-register via the terminal.
if [ -n "$BPMUX_BIN" ] && [ -n "${BPMUX_ROLE:-}" ]; then
    "$BPMUX_BIN" register >/dev/null 2>&1 || true
fi

# Liveness keepalive (#1721): an idle off-MCP agent fires no hooks and makes no
# calls between turns, so the server cannot see it and would reap its roster row.
# Spawn a detached keepalive that pings on a cadence while THIS agent process is
# alive and exits the moment it dies — so a killed agent stops signalling and the
# server reaps it past the grace (the ungraceful-death backstop SessionEnd cannot
# cover). The agent PID is discovered from the transcript file the agent holds open
# (Claude Code exposes no PID, and a hook's parent PID is not a documented handle);
# the parent PID is a last resort. nohup detaches it from this short-lived hook
# (macOS has no setsid) so it survives the hook's return.
if [ -n "$BPMUX_BIN" ] && [ -n "${BPMUX_ROLE:-}" ]; then
    transcript_path=$(printf '%s' "$HOOK_INPUT" \
        | grep -o '"transcript_path"[[:space:]]*:[[:space:]]*"[^"]*"' \
        | sed 's/.*:[[:space:]]*"//; s/"$//')
    agent_pid=""
    if [ -n "$transcript_path" ]; then
        agent_pid=$(lsof -t -- "$transcript_path" 2>/dev/null | head -1)
    fi
    [ -z "$agent_pid" ] && agent_pid="$PPID"
    if [ -n "$agent_pid" ]; then
        nohup "$BPMUX_BIN" keepalive --watch-pid "$agent_pid" >/dev/null 2>&1 &
        keepalive_pid=$!
        disown 2>/dev/null || true
        # Record the keepalive PID keyed by session-id so the SessionEnd hook can
        # kill it before the clean-disconnect — otherwise an in-flight ping could
        # resurrect a just-disconnected agent. session-id is stable across the two
        # hooks (it is in both payloads).
        session_id=$(printf '%s' "$HOOK_INPUT" \
            | grep -o '"session_id"[[:space:]]*:[[:space:]]*"[^"]*"' \
            | sed 's/.*:[[:space:]]*"//; s/"$//')
        if [ -n "$session_id" ]; then
            printf '%s' "$keepalive_pid" > "${TMPDIR:-/tmp}/bpmux-keepalive-${session_id}.pid" 2>/dev/null || true
        fi
    fi
fi

if [ -n "$BPMUX_BIN" ] && [ -n "${BPMUX_ROLE:-}" ]; then
    echo ""
    "$BPMUX_BIN" recall --role "$BPMUX_ROLE" 2>/dev/null || true
fi

# Post-compact briefing experiment (2026-04-17 iteration).
# Agent-authored continuity note at ~/.bpmux/briefings/{role}.md; read
# on every session start and emitted alongside lesson recall. Missing/
# empty briefing is a silent no-op. mtime-derived relative age gives
# the post-compact self a "when was I last active on this" signal.
# If the experiment validates, productized via the shaped story (#287
# or equivalent) with MCP save_briefing / get_briefing / view_briefings
# tools replacing manual file authoring.
if [ -n "${BPMUX_ROLE:-}" ]; then
    briefing_file="$HOME/.bpmux/briefings/${BPMUX_ROLE}.md"
    if [ -s "$briefing_file" ]; then
        # macOS uses `stat -f %m`, Linux uses `stat -c %Y`. Try macOS
        # first and fall back so the hook works on either platform.
        mtime=$(stat -f %m "$briefing_file" 2>/dev/null || stat -c %Y "$briefing_file" 2>/dev/null || echo 0)
        now=$(date +%s)
        age=$((now - mtime))
        if [ "$age" -lt 60 ]; then rel="just now"
        elif [ "$age" -lt 3600 ]; then rel="$((age / 60)) minutes ago"
        elif [ "$age" -lt 86400 ]; then rel="$((age / 3600)) hours ago"
        else rel="$((age / 86400)) days ago"
        fi
        printf '\n## briefing for %s (last written: %s)\n\n' "$BPMUX_ROLE" "$rel"
        cat "$briefing_file"
    fi
fi

exit 0
