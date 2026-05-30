#!/usr/bin/env bash
# Test suite for filing-diagnosis-as-fix-gate.sh.
# Pins: (a) the grab-bag class (remediation + plural-symptom, no root-cause)
# is DENIED; (b) the false-positive contract — root-cause-named / spike /
# normal-feature / single-named-bug / "fix" in feature-prose — ALLOWS.
# Run via: bash tests/test_filing_diagnosis_gate.sh
set -u

HOOK="$(cd "$(dirname "$0")/.." && pwd)/filing-diagnosis-as-fix-gate.sh"
PASS=0
FAIL=0

run_case() {
  local label="$1"
  local title="$2"
  local user_job="$3"
  local body="$4"
  local expect="$5"  # "allow" | "deny"
  local input
  input=$(jq -n --arg t "$title" --arg u "$user_job" --arg b "$body" \
    '{"tool_name":"mcp__bpmux-bridge__create_story","tool_input":{"title":$t,"user_job":$u,"body":$b}}')
  local out got
  out=$(echo "$input" | "$HOOK")
  if [ -z "$out" ]; then
    got="allow"
  elif echo "$out" | grep -q '"permissionDecision": "deny"'; then
    got="deny"
  else
    got="unknown"
  fi
  if [ "$got" = "$expect" ]; then
    echo "PASS  [$label] -> $got"
    PASS=$((PASS+1))
  else
    echo "FAIL  [$label] expected=$expect got=$got"
    echo "      out:  $out"
    FAIL=$((FAIL+1))
  fi
}

# ── DENY: the grab-bag class (remediation + plural-symptom, no root-cause) ──
run_case "1583-the-actual-grab-bag" \
  "[Chore] Stabilize the real-backend Playwright flake-clusters (timing/isolation races)" \
  "When dev pushes, the suite shouldn't flap, because flaky tests block pushes" \
  "Stabilize the flake-clusters blocking pushes." \
  "deny"
run_case "fix-all-failing-tests" \
  "[Chore] Fix all the failing tests" \
  "the team needs the failing tests green" \
  "Fix all the failures in the e2e suite." \
  "deny"
run_case "harden-flaky-suite" \
  "[Chore] Harden the flaky e2e suite" \
  "make the flaky suite reliable" \
  "Harden against the flakes and timeouts." \
  "deny"
run_case "deflake-races" \
  "[Chore] De-flake the room races" \
  "the room races cause failures" \
  "deflake the races and crashes." \
  "deny"

# ── ALLOW: remediation WITH enumerated root-causes (the escape hatch) ──
run_case "stabilize-with-root-cause" \
  "[Bug] Designate intermittently 400s" \
  "When I create then designate an iteration, it should reliably succeed" \
  "Root cause: create_iteration attaches the iteration label via an async TagAdded, so designate reads it before the label drains. Fix: attach atomically at creation." \
  "allow"

# ── ALLOW: explicitly a spike ──
run_case "spike-investigate" \
  "[Spike] Investigate the flake-clusters" \
  "we need to know what's causing the flakes before we can fix them" \
  "Timeboxed diagnosis. Filing-class: Spike. [spike] — surface the real fix-stories." \
  "allow"

# ── ALLOW: normal Feature (no remediation+symptom) ──
run_case "normal-feature-terminal" \
  "[Feature] Click an agent's name → the chat-area becomes the live terminal" \
  "As Ryan, I want to click an agent and see its live terminal, so I can watch it" \
  "Render the PTY in the chat-area." \
  "allow"

# ── ALLOW: single named bug (singular symptom, not a cluster) ──
run_case "single-named-bug" \
  "[Bug] Fix the login button crash" \
  "When I click login, it shouldn't crash" \
  "Fix the crash on the login button." \
  "allow"

# ── ALLOW: "fix" inside feature-prose (not a remediation-of-a-cluster) ──
run_case "fix-in-feature-prose" \
  "[Feature] Add a retry button for failed uploads" \
  "As a user, I want to retry so I can fix failed uploads myself" \
  "Add a retry affordance." \
  "allow"

# ── ALLOW: legit specific test-fix that names its sites (the #1591 shape) ──
run_case "specific-test-fix-named" \
  "[Chore] Web-first the non-retrying one-shot reads in human-board specs" \
  "the named reads snapshot a half-hydrated tree" \
  "Convert scroll-and-sticky:6 and count-badge:77/148 to web-first matchers. Root cause: non-retrying .count() reads observe a half-rendered tree under hydration." \
  "allow"

# ── ALLOW: empty / non-create_story ──
run_case "empty-fields" "" "" "" "allow"

echo
echo "==================================="
echo "PASS: $PASS  FAIL: $FAIL"
echo "==================================="
exit $FAIL
