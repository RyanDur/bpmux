#!/usr/bin/env bash
# filing-premise-reminder.sh — emit a premise-check reminder on
# create_story (#290).
#
# Fires when Claude Code is about to invoke `mcp__bpmux-bridge__create_story`,
# per a PreToolUse matcher in hooks.json. Emits `{systemMessage: "..."}`
# on stdout so Claude Code injects the reminder into the agent's
# context before the story is filed. Non-blocking: the hook never
# prevents filing; it just surfaces the premise-check questions at
# the right moment.
#
# Why: tonight's most-observed drift was PM-lane filing-premise
# verification — 5 unverified-premise filings within ~3 hours (#279,
# team-docs recreation, #211 almost, stretch candidates). Saved
# lesson `feedback_verify_premise_before_filing` exists but wasn't
# firing at the right moment. Today's reminder is owner's
# "objectively?"; this hook automates it at the create_story moment.
#
# Content is role-agnostic in v1 — the four premise questions apply
# to any filing role. Future versions can add role-flavor if specific
# drift patterns emerge per role.

set -euo pipefail

role="${BPMUX_ROLE:-}"

# Unknown / unset role → silent no-op. Can't contextualize the
# reminder without knowing who's filing.
if [ -z "$role" ]; then
  exit 0
fi

# Same lens content for pm / dev / designer. Filing discipline is
# universal; adjusting for role didn't add signal in the story's
# observed-drift data (5 misses were spread across pm + dev + me
# at various filing moments).
case "$role" in
  pm|dev|designer) ;;
  *) exit 0 ;;
esac

lenses="observed pain or speculation? (speculative → don't file) / substrate verified? (grep code, read related docs, git history for prior decisions) / reverses a deliberate past decision? (if yes, escalate to owner with context) / extends existing or duplicates? (check supersession risk)"

message="[file] premise check before filing for ${role}: ${lenses}"

escaped="${message//\\/\\\\}"
escaped="${escaped//\"/\\\"}"

printf '{"systemMessage":"%s"}\n' "$escaped"
