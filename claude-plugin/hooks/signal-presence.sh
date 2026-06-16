#!/bin/bash
# signal-presence.sh — pushes this agent's collaboration-dot state via S1's
# gated /api/presence path (the `bpmux signal` verb), OFF the chat stream.
# Wired per-agent-type by hooks.json with zero per-agent setup:
#   UserPromptSubmit -> busy   (turn start)
#   Stop             -> idle   (definitive turn end)
#   Notification     -> help, BUT only for a genuine permission_prompt.
#
# The Notification event ALSO fires on a ~60s idle-waiting (notification_type
# "idle_prompt"), not just permission asks. Mapping every Notification to help
# false-reds the dot as "needs-you" while the agent is merely idle — the exact
# dishonesty this feature exists to remove. So for the help case we read the
# payload and keep help ONLY for a permission_prompt; anything else (idle_prompt,
# any other type, or no field) stays idle and never false-reds.
#
# Pure-bash glob match (no jq dependency, so it can't break in a hook env), and
# idle is the safe default. Best-effort: a failed presence push must never block
# the turn.

state="$1"
if [ "$state" = "help" ]; then
  case "$(cat 2>/dev/null)" in
    *permission_prompt*) state="help" ;;
    *) state="idle" ;;
  esac
fi
bpmux signal "$state" >/dev/null 2>&1
exit 0
