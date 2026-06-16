#!/bin/bash
# signal-presence.sh — pushes this agent's collaboration-dot state via S1's
# gated /api/presence path (the `bpmux signal` verb), OFF the chat stream.
# Wired per-agent-type by hooks.json with zero per-agent setup:
#   UserPromptSubmit -> busy   (turn start)
#   Stop             -> idle   (definitive turn end)
#   Notification     -> help   (the agent needs the owner)
# Replaces the retired set-status-*.sh, which POSTed presence into the chat
# message stream (spam). Best-effort: a failed presence push must never block the turn.

state="$1"
bpmux signal "$state" >/dev/null 2>&1
exit 0
