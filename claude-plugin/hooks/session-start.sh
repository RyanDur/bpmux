#!/bin/bash
# session-start.sh — SessionStart hook
#
# 1. Reminds Claude that bpmux is connected (bridge identity + status rules).
# 2. Surfaces the agent's own per-role lessons from prior sessions so bpmux's
#    lesson index becomes the authoritative auto-loaded memory path (#157).
#    Closes the two-store duplication with Claude Code's auto-memory.
#
# The recall call reads directly from ~/.bpmux/sessions/current/lessons.json
# (via `bpmux recall`), so it works even if the bpmux TUI isn't running yet.
# Role comes from BPMUX_ROLE, which each workspace's .claude/settings.local.json
# already sets. Missing role → recall silently emits a "no prior lessons"
# line, which is fine.

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

if [ -n "$BPMUX_BIN" ] && [ -n "${BPMUX_ROLE:-}" ]; then
    echo ""
    "$BPMUX_BIN" recall --role "$BPMUX_ROLE" 2>/dev/null || true
fi

exit 0
