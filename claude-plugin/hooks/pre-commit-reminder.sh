#!/usr/bin/env bash
# pre-commit-reminder.sh — emit a pre-commit discipline reminder
# when dev fires `git commit` (#538).
#
# Fires when Claude Code is about to invoke a Bash command that
# starts with `git commit`, per a PreToolUse matcher in hooks.json.
# Emits `{systemMessage: "..."}` on stdout so Claude Code injects
# the reminder into the agent's context BEFORE the commit lands.
# Non-blocking — never prevents the commit; just surfaces the
# discipline lenses at the right moment.
#
# Why: per #538 cross-role iteration-close audit (2026-04-28), dev
# bypassed pre-commit-review subagent during high-throughput commit
# cycles. Per `feedback_external_catch_over_self_catch`, substrate-
# layer external-catch closes the skill-bypass class structurally.
# Today's `feedback_real_time_recidivism_means_substrate` reinforces:
# discipline-only-fix won't hold under density.
#
# Density-detection: only fires when commits arrive in a tight
# burst (sub-N-minute cadence between consecutive commits). Slow
# isolated commits don't get hook-noise per the spec's class-target
# at HIGH-THROUGHPUT.

set -euo pipefail

role="${BPMUX_ROLE:-}"

# Unknown / unset role → silent no-op.
if [ -z "$role" ]; then
  exit 0
fi

# Only dev commits via this lane. Other roles can edit settings /
# memory / docs but typically don't run code-commit ceremony.
case "$role" in
  dev) ;;
  *) exit 0 ;;
esac

# Read the bash command from stdin JSON to confirm this is actually
# a `git commit` call (not e.g. `git commit-tree` or other variants).
input=$(cat 2>/dev/null || true)
command_line=$(printf '%s' "$input" | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
    print(d.get("tool_input", {}).get("command", ""), end="")
except Exception:
    pass
' 2>/dev/null || true)

# Match git commit but not git commit-tree / git commit --help / etc.
if ! printf '%s' "$command_line" | grep -qE '(^|[[:space:]/])git[[:space:]]+(-C[[:space:]]+[^[:space:]]+[[:space:]]+)?commit([[:space:]]|$)'; then
  exit 0
fi

# Density-detection — only fire when this commit follows a recent
# prior commit. State file at /tmp/bpmux-pre-commit-density-${role}.
# Default window 5 minutes (per PM 2026-05-05 02:43 — commit bursts
# often arrive 2-5min apart). Env-overridable via
# BPMUX_HOOK_COMMIT_DENSITY_WINDOW_SECS so tuning happens without
# recompile (per designer 02:42).
state_file="${BPMUX_HOOK_COMMIT_DENSITY_STATE_FILE:-/tmp/bpmux-pre-commit-density-${role}}"
window_secs="${BPMUX_HOOK_COMMIT_DENSITY_WINDOW_SECS:-300}"
now=$(date +%s)
last_ts=0
if [ -f "$state_file" ]; then
  last_ts=$(cat "$state_file" 2>/dev/null || echo 0)
fi
printf '%s' "$now" > "$state_file"
elapsed=$((now - last_ts))
if [ "$last_ts" -eq 0 ] || [ "$elapsed" -gt "$window_secs" ]; then
  # Slow / isolated commit — no hook noise per class-target rule.
  exit 0
fi

lenses='debt-at-sight-scan (touched files have visible smells?) / tidy-first-preflight (refactor entangled with feature?) / pre-commit-review (zero-inline-comments / scope-mismatch / hidden-debt) / scope = one commit one concern (feedback_story_scope_equals_commit_scope)'

message="[commit] high-throughput pre-commit lenses for ${role} (${elapsed}s since last commit): ${lenses}"

escaped="${message//\\/\\\\}"
escaped="${escaped//\"/\\\"}"

printf '{"systemMessage":"%s"}\n' "$escaped"
