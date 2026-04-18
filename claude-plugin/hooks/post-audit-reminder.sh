#!/usr/bin/env bash
# post-audit-reminder.sh — emit a role-appropriate post-audit subagent
# reminder on deliver_story (#289).
#
# Fires when Claude Code is about to invoke `mcp__bpmux-bridge__deliver_story`,
# per a PreToolUse matcher in hooks.json. Emits `{systemMessage: "..."}`
# on stdout so Claude Code injects the reminder into the agent's
# context before delivery completes. Non-blocking by design — the
# reminder never prevents the tool from executing.
#
# Why: dev shipped #286 with `let _ = ws_send(...).await` silently
# discarding a Result (auto-FAIL gate at desk-check). A post-audit
# lens run at deliver_story would have caught it before delivery.
# Tonight's pattern: post-audit skipped on #280 / #282 / #283 / #286
# because changes felt "obviously small." This hook automates the
# reminder at the delivery moment so the discipline doesn't rely on
# memory.

set -euo pipefail

role="${BPMUX_ROLE:-}"

if [ -z "$role" ]; then
  exit 0
fi

case "$role" in
  dev)
    lenses="correctness / TDD coverage / security (role-gated or broadcast paths) / debt-at-sight (let _, variance-tolerant asserts, follow-up-story phrasings)"
    ;;
  designer)
    # Designer's deliver_story fires for designer-shipped stories —
    # fix-verification is the lens (did the shipped change match the AC?).
    lenses="fix-verification (shipped change matches AC? premise-rework landed where it needed to?)"
    ;;
  pm|*)
    # PM doesn't ship code via deliver_story per the role contract;
    # unknown roles also get silence. No reminder → silent no-op.
    exit 0
    ;;
esac

message="[deliver] post-audit subagent lenses for ${role}: ${lenses}. Run the set, never a subset."

escaped="${message//\\/\\\\}"
escaped="${escaped//\"/\\\"}"

printf '{"systemMessage":"%s"}\n' "$escaped"
