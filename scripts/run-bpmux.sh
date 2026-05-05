#!/usr/bin/env bash
# run-bpmux.sh — supervisor for bpmux (#118 self-update).
#
# Loops bpmux's lifecycle: each iteration spawns the binary; if it
# exits with code 200, the binary detected its own file changed on
# disk and asked to be respawned (the new binary picks up the next
# iteration). Any other exit code terminates the loop — clean stop
# (0), error (1), killed (137), etc. all break out.
#
# Restart-loop guard: if the binary respawns N times within a
# window (default 5 in 60 seconds), halt with an explicit non-zero
# exit so the operator catches a bad-binary thrash instead of
# silent CPU/disk burn. Tunable via env vars
# BPMUX_RESTART_MAX_COUNT and BPMUX_RESTART_WINDOW_SECS.
#
# Usage:
#   run-bpmux.sh [bpmux-args...]
#
# Example:
#   run-bpmux.sh serve --mcp-port 4117

set -uo pipefail

BPMUX_BIN="${BPMUX_BIN:-bpmux}"
RESTART_EXIT_CODE=200
MAX_RESTARTS="${BPMUX_RESTART_MAX_COUNT:-5}"
WINDOW_SECS="${BPMUX_RESTART_WINDOW_SECS:-60}"
HALT_EXIT_CODE=99

restarts=0
window_start=$(date +%s)

while true; do
  "$BPMUX_BIN" "$@"
  code=$?
  if [ "$code" -ne "$RESTART_EXIT_CODE" ]; then
    exit "$code"
  fi
  now=$(date +%s)
  if [ $((now - window_start)) -gt "$WINDOW_SECS" ]; then
    restarts=0
    window_start=$now
  fi
  restarts=$((restarts + 1))
  if [ "$restarts" -ge "$MAX_RESTARTS" ]; then
    echo "[run-bpmux.sh] $restarts restarts in <${WINDOW_SECS}s — halting (likely bad binary; check build)" >&2
    exit "$HALT_EXIT_CODE"
  fi
  echo "[run-bpmux.sh] bpmux exited $RESTART_EXIT_CODE (binary changed); respawning ($restarts/$MAX_RESTARTS within ${WINDOW_SECS}s window)" >&2
done
