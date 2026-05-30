#!/usr/bin/env bash
# filing-diagnosis-as-fix-gate.sh — DENY a create_story whose framing is
# SYMPTOM-REMEDIATION without enumerated root-causes (route to SPIKE).
#
# The grab-bag class, caught at FILING (Dorado #1583: "[Chore] Stabilize
# the real-backend Playwright flake-CLUSTERS / timing-isolation RACES"
# was filed as ONE chore but hid THREE distinct root causes — split only
# at dev's mid-flight diff-review, the expensive place). story-shape.md
# §5: a symptom-remediation whose distinct root causes can't be named
# without investigating is a SPIKE, not a fix-slice.
#
# Fires on: mcp__bpmux-bridge__create_story (PreToolUse), any role —
# the #1583 grab-bag was a dev Chore, so this is class-agnostic.
#
# DENY when: a REMEDIATION verb AND a (typically plural) SYMPTOM-noun are
# both present, UNLESS enumerated root-cause markers (root cause / caused
# by / stems from) or a [spike] marker appear.
#   - "because" is NOT a root-cause marker: the user_job format
#     ("When X, role needs Y because Z") mandates it → would false-allow.
#   - DENY (not ask): per feedback_hooks_deny_not_ask — the author reads
#     the guidance and self-corrects (enumerate root-causes, or reframe as
#     a spike) without waiting on owner-permission. A fix WITH named
#     root-causes passes the gate.

if ! command -v jq &>/dev/null; then
  exit 0
fi

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

if [ "$TOOL_NAME" != "mcp__bpmux-bridge__create_story" ]; then
  exit 0
fi

# The framing lives in title + user_job + body.
TEXT=$(echo "$INPUT" | jq -r '[.tool_input.title, .tool_input.user_job, .tool_input.body] | map(. // "") | join(" ")')
if [ -z "${TEXT// /}" ]; then
  exit 0
fi

LOWER=$(echo "$TEXT" | tr '[:upper:]' '[:lower:]')

# Remediation verbs (the "make-the-bad-thing-stop" framing).
REMEDIATION_PATTERN='stabili[sz]e|de-?flake|unbreak|harden|stamp out|fix the|fix our|fix all|clean ?up|cleanup'
# Plural symptom-nouns (the bundle-tell — a CLUSTER of bad things, not one
# named defect). Plural-leaning on purpose: "fix the crash" (one named bug)
# must pass; "stabilize the crashes/flakes/races" (a cluster) is the catch.
SYMPTOM_PATTERN='flaky|flakes|flake-?cluster|races|race-conditions|failures|failing tests|errors|leaks|hangs|timeouts|crashes|regressions|instabilit'

if ! echo "$LOWER" | grep -qE "$REMEDIATION_PATTERN"; then
  exit 0
fi
if ! echo "$LOWER" | grep -qE "$SYMPTOM_PATTERN"; then
  exit 0
fi

# Escape hatch: root-causes named, or explicitly a spike. NOT "because".
ROOTCAUSE_PATTERN='root[ -]cause|caused by|stems from|root[ -]causes:|\[spike\]|^spike|filing-class.{0,20}spike'
if echo "$LOWER" | grep -qE "$ROOTCAUSE_PATTERN"; then
  exit 0
fi

REASON="This filing reads as SYMPTOM-REMEDIATION without enumerated root-causes — the grab-bag class (story-shape.md §5; Dorado #1583 \"stabilize the flake-clusters\" hid 3 distinct root causes, split only at pickup). Either: (1) NAME the distinct root causes in the body — then file ONE fix-slice per root cause (each its own vertical); or (2) if the root causes can't be named without investigating, file a SPIKE (diagnose -> surface the real fix-stories) and mark it [spike]. A fix WITH named root-causes passes this gate. Site-enumerability is not root-cause-enumerability: namable failing tests can still hide multiple unknown causes."

jq -n \
  --arg reason "$REASON" \
  '{
    "hookSpecificOutput": {
      "hookEventName": "PreToolUse",
      "permissionDecision": "deny",
      "permissionDecisionReason": $reason
    }
  }'
