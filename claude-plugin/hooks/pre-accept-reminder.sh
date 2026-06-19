#!/usr/bin/env bash
# pre-accept-reminder.sh — emit a pre-accept quality-audit reminder
# on accept_story (#538).
#
# Fires when Claude Code is about to invoke
# `mcp__bpmux-bridge__accept_story`, per a PreToolUse matcher in
# hooks.json. Emits `{systemMessage: "..."}` on stdout so Claude
# Code injects the reminder into the agent's context BEFORE the
# accept fires. Non-blocking — never prevents acceptance, just
# surfaces the audit lens at the right moment.
#
# Why: per #538 cross-role iteration-close audit (2026-04-28), PM
# filed false-alarm stories (#532, #535) without invoking the
# pre-accept-quality-audit / pre-accept-live-walk skills. Per
# `feedback_external_catch_over_self_catch`, substrate-layer reminder
# at the moment-of-action closes the skill-existence-vs-application
# gap structurally. Today's `feedback_quality_is_paramount` +
# `feedback_bug_question_test_at_desk_check` lessons compound: the
# moment-of-acceptance is exactly where these lenses must fire.

set -euo pipefail

role="${BPMUX_ROLE:-}"

# Unknown / unset role → silent no-op. accept_story is owner-only
# (owner), so missing role-context means we can't tailor the lens.
if [ -z "$role" ]; then
  exit 0
fi

# Only fire for the lane that calls accept_story. PM acts as
# proxy-accept on owner's behalf today; owner accepts directly via
# Electron. Both surfaces are governed by the same audit-lens.
case "$role" in
  pm|owner) ;;
  *) exit 0 ;;
esac

# Density-detection — only fire when accept arrives in a burst (PM
# 2026-05-05 02:43 named 10min as the right window for accepts).
# Env-overridable via BPMUX_HOOK_ACCEPT_DENSITY_WINDOW_SECS per
# designer's tuning-without-recompile request.
state_file="${BPMUX_HOOK_ACCEPT_DENSITY_STATE_FILE:-/tmp/bpmux-pre-accept-density-${role}}"
window_secs="${BPMUX_HOOK_ACCEPT_DENSITY_WINDOW_SECS:-600}"
now=$(date +%s)
last_ts=0
if [ -f "$state_file" ]; then
  last_ts=$(cat "$state_file" 2>/dev/null || echo 0)
fi
printf '%s' "$now" > "$state_file"
elapsed=$((now - last_ts))
if [ "$last_ts" -eq 0 ] || [ "$elapsed" -gt "$window_secs" ]; then
  exit 0
fi

lenses='ci green? (bpmux get_ci_state — red blocks regardless of root cause) / live-walk done? (the path the user-job names, not just AC mechanism) / would I file a Bug if observed? (bug-question test at acceptance) / scope = user-job satisfied, not AC-checkbox-mechanism (feedback_accept_user_story_not_ac_checkbox)'

message="[accept] high-throughput pre-accept lenses for ${role} (${elapsed}s since last accept): ${lenses}"

escaped="${message//\\/\\\\}"
escaped="${escaped//\"/\\\"}"

printf '{"systemMessage":"%s"}\n' "$escaped"
