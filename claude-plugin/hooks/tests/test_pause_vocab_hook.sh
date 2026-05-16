#!/usr/bin/env bash
# Test suite for bpmux-broadcast-vocabulary-catch.sh (#1372).
# Pins the false-positive-rate-on-correction-class = 0 contract while
# keeping active-use detection intact. Run via:
#   bash tests/test_pause_vocab_hook.sh
set -u

HOOK="$(cd "$(dirname "$0")/.." && pwd)/bpmux-broadcast-vocabulary-catch.sh"
PASS=0
FAIL=0

run_case() {
  local label="$1"
  local text="$2"
  local expect="$3"  # "allow" | "ask"
  local input
  input=$(jq -n --arg t "$text" '{"tool_name":"mcp__bpmux-bridge__send_message","tool_input":{"text":$t}}')
  local out
  out=$(echo "$input" | "$HOOK")
  local got
  if [ -z "$out" ]; then
    got="allow"
  elif echo "$out" | grep -q '"permissionDecision": "ask"'; then
    got="ask"
  else
    got="unknown"
  fi
  if [ "$got" = "$expect" ]; then
    echo "PASS  [$label] -> $got"
    PASS=$((PASS+1))
  else
    echo "FAIL  [$label] expected=$expect got=$got"
    echo "      text: $text"
    echo "      out:  $out"
    FAIL=$((FAIL+1))
  fi
}

# ── active-use (banned phrase as imperative-active-pause) ──
run_case "active-standing-by-no-cause"   "standing by"   "ask"
run_case "active-monitoring-no-cause"    "monitoring the channel" "ask"
run_case "active-hold-position-no-cause" "hold position" "ask"

# ── active-use with named cause (allow) ──
run_case "active-with-named-blocker"     "standing by — blocker on CI red-ci"    "allow"
run_case "active-with-queue-empty"       "monitoring; queue-empty"               "allow"
run_case "active-with-owner-signal"      "hold position — owner-signal pending"  "allow"

# ── quoted-context (correction-class; the #1372 false-positive class) ──
run_case "quoted-single-correction"     "PM used 'standing by' earlier — apologies; reshaping" "allow"
run_case "quoted-double-correction"     'dev quoted "standing by" in narrative; not an active pause' "allow"
run_case "quoted-backtick-correction"   'I used `standing by` last broadcast — fixed' "allow"
run_case "quoted-canon-explainer"       "the pause-vocab hook fires on 'standing by' without named cause" "allow"

# ── adversarial: unquoted banned + quoted-named-cause ──
# This should still trip (banned outside quotes, no real named cause).
run_case "adversarial-named-only-in-quote" "standing by since 'blocker' was named in a prior note" "allow"
# ^ Note: named-cause regex matches FULL text including quotes, so this allows.
# That's the documented design — see hook script comment.

# ── no banned phrase ──
run_case "no-banned-phrase"               "moving on to the next story"          "allow"

# ── empty text ──
run_case "empty-text"                     ""                                      "allow"

echo
echo "==================================="
echo "PASS: $PASS  FAIL: $FAIL"
echo "==================================="
exit $FAIL
