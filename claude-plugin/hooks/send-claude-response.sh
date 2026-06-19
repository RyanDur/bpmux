#!/bin/bash
# send-claude-response.sh — Stop hook
#
# Surfaces new bpmux messages into the agent's conversation context.
#
# **Historical shape (pre-#185)**: injected only a count of new
# messages — e.g. `"3 new bpmux messages arrived."` — which relied
# on the agent calling `read_messages` to fetch content. This
# created a silent failure mode: if the incoming `← bpmux-bridge`
# render drops a message in claude-code's terminal AND the agent
# doesn't proactively read, the message exists in the bridge log
# but never reaches the agent's working context. Both pm and dev
# sessions hit this during today's session (#185).
#
# **Current shape (#185 fix)**: injects the full text of each new
# message into the systemMessage body so the agent's conversation
# state receives the content directly, independent of the terminal
# render layer. The render layer is best-effort display; the hook
# injection is the authoritative delivery path.
#
# **Testing**: set `BPMUX_MESSAGES_STDIN=1` and pipe a messages
# JSON array on stdin instead of curling the live API. See
# `applications/bpmux/tests/send_claude_response_hook_test.rs` for the integration
# test that exercises the formatting logic against known inputs.

BPMUX_PORT="${BPMUX_PORT:-4117}"
BPMUX_API="http://127.0.0.1:${BPMUX_PORT}/api/messages"
LAST_SEEN_FILE="${BPMUX_LAST_SEEN_FILE:-/tmp/bpmux-last-seen-$PPID}"

if ! command -v jq >/dev/null 2>&1; then
  exit 0
fi

# Message source: stdin (test mode) or live API (production).
if [ "${BPMUX_MESSAGES_STDIN:-0}" = "1" ]; then
  MESSAGES=$(cat)
else
  MESSAGES=$(curl -s "$BPMUX_API" 2>/dev/null)
fi

if [ -z "$MESSAGES" ] || [ "$MESSAGES" = "null" ]; then
  exit 0
fi

LAST_SEEN=""
if [ -f "$LAST_SEEN_FILE" ]; then
  LAST_SEEN=$(cat "$LAST_SEEN_FILE" 2>/dev/null)
fi

# First run — mark position, don't inject.
if [ -z "$LAST_SEEN" ]; then
  echo "$MESSAGES" | jq -r '.[-1].ts // empty' > "$LAST_SEEN_FILE" 2>/dev/null
  exit 0
fi

# Collect the full new-message objects (chat-type only, excluding
# owner's own messages so the agent doesn't echo-read its own
# prompts back through the hook).
NEW_MESSAGES=$(echo "$MESSAGES" | jq --arg last "$LAST_SEEN" '
  [.[] |
    select(.ts > $last) |
    select((.message_type // "chat") == "chat") |
    select(.from | test("owner:claude-code|owner:mcp-client") | not)
  ]
')

NEW_COUNT=$(echo "$NEW_MESSAGES" | jq 'length' 2>/dev/null)

# Update last seen to the most recent message in the log (not just
# the filtered new-messages set) so we advance even if only owner's
# own messages were appended.
echo "$MESSAGES" | jq -r '.[-1].ts // empty' > "$LAST_SEEN_FILE" 2>/dev/null

if [ "$NEW_COUNT" = "0" ] || [ -z "$NEW_COUNT" ]; then
  exit 0
fi

# Format each new message as `[role] text`, one per line, with the
# content joined into a single block under a header. `role` is the
# short prefix before `:` in the `from` field. Text is included
# verbatim — no truncation — because truncating would defeat the
# whole point of the fix. If an individual message is very large
# the host will handle it; the hook's job is delivery, not size.
BODY=$(echo "$NEW_MESSAGES" | jq -r '
  map("[" + (.from | split(":")[0]) + "] " + .text)
  | join("\n\n")
')

# Build the systemMessage header + body. `continue: true` keeps
# claude-code in the turn so the agent sees the content immediately.
jq -nc --arg count "$NEW_COUNT" --arg body "$BODY" '{
  "continue": true,
  "systemMessage": ($count + " new bpmux message" + (if $count == "1" then "" else "s" end) + ":\n\n" + $body)
}'

exit 0
