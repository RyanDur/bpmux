#!/usr/bin/env bash
# preflight-reminder.sh — emit a role-appropriate pre-flight subagent
# reminder on start_story (#288).
#
# Fires when Claude Code is about to invoke `mcp__bpmux-bridge__start_story`,
# per a PreToolUse matcher in hooks.json. Emits `{systemMessage: "..."}`
# on stdout so Claude Code injects the reminder into the agent's
# context before the tool call proceeds. Non-blocking by design —
# the reminder never prevents the tool from executing.
#
# Why: dev drifted on pre-flight subagents 3x in the 2026-04-17
# iteration (#280, #282, #283) because changes felt "obviously
# small." Caught reactively after Ryan asked "still using subagents?"
# This hook automates the reminder at the pickup moment so the
# discipline doesn't rely on memory.

set -euo pipefail

role="${BPMUX_ROLE:-}"

# Unknown / unset role → silent no-op. A systemMessage emit with
# empty or generic content would be noise; prefer nothing.
if [ -z "$role" ]; then
  exit 0
fi

case "$role" in
  dev)
    lenses="architecture (module-graph fit) / tidy (debt in touched files) / test-plan (AC-derived; do tests exist?) / UX-check (user-visible behavior → designer consult)"
    ;;
  designer)
    lenses="convergence test (three independent reads → same spec) / substrate verification (proposed infra actually exists in code)"
    ;;
  pm)
    lenses="filing-premise-verify (grep substrate before filing) / inventory existing bindings (if extending UI, what's already bound?)"
    ;;
  *)
    # Role recognized by bpmux but no configured lens list — silent.
    exit 0
    ;;
esac

message="[pickup] pre-flight subagent lenses for ${role}: ${lenses}. Run the set, never a subset."

# systemMessage JSON must be a single line of valid JSON. Escape
# backslashes and double-quotes in the message so the JSON stays
# well-formed even if the lens list ever grows quote characters.
escaped="${message//\\/\\\\}"
escaped="${escaped//\"/\\\"}"

printf '{"systemMessage":"%s"}\n' "$escaped"
