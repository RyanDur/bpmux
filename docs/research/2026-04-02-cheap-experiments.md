# Cheap Experiments: AI-as-Participant vs AI-as-Tool

Date: 2026-04-02

## The Hypothesis

An AI that participates in a group chat conversation is more valuable for pair programming than an AI you invoke as a tool.

## What We Already Have

bpmux writes messages to `~/.bpmux/sessions/<uuid>/messages-current.jsonl`. Each line:

```json
{"from":"ryan:human","text":"hello world","ts":"2026-04-02T12:00:00Z"}
```

The MCP server (port-based, Streamable HTTP) exposes two tools: `read_messages` (returns full history as JSON array) and `send_message` (appends to JSONL, wakes the TUI via broadcast channel). Messages from MCP clients arrive with `from: "<peer>:mcp-client"`.

---

## Experiment 1: The Shell Script Bridge

**Cost:** A few hours. No Rust changes.

**How it works:** A bash script tails the JSONL file, detects new human messages, sends the conversation to the Anthropic Messages API, and writes the response back by calling bpmux's MCP `send_message` tool.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Configuration
BPMUX_MCP_URL="http://127.0.0.1:8080/mcp"
ANTHROPIC_API_KEY="${ANTHROPIC_API_KEY:?Set ANTHROPIC_API_KEY}"
MODEL="claude-sonnet-4-20250514"
AI_NAME="claude"
POLL_INTERVAL=2

# Find the current session's JSONL
SESSION_DIR="$(readlink ~/.bpmux/sessions/current)"
JSONL="${SESSION_DIR}/messages-current.jsonl"

last_line_count=0
if [[ -f "$JSONL" ]]; then
  last_line_count=$(wc -l < "$JSONL" | tr -d ' ')
fi

echo "Watching ${JSONL} (starting after line ${last_line_count})"

while true; do
  sleep "$POLL_INTERVAL"

  [[ -f "$JSONL" ]] || continue
  current_count=$(wc -l < "$JSONL" | tr -d ' ')
  [[ "$current_count" -le "$last_line_count" ]] && continue

  # Read new lines
  new_lines=$(tail -n +"$((last_line_count + 1))" "$JSONL")
  last_line_count="$current_count"

  # Skip if the newest message is from the AI (avoid reply loops)
  last_from=$(echo "$new_lines" | tail -1 | jq -r '.from')
  [[ "$last_from" == *":mcp-client"* ]] && continue

  # Build the messages array from full history
  messages=$(jq -s '
    [.[] | {
      role: (if (.from | test(":human$")) then "user" else "assistant" end),
      content: .text
    }]
  ' "$JSONL")

  # Call the Anthropic Messages API
  response=$(curl -s https://api.anthropic.com/v1/messages \
    -H "x-api-key: ${ANTHROPIC_API_KEY}" \
    -H "anthropic-version: 2023-06-01" \
    -H "content-type: application/json" \
    -d "$(jq -n \
      --arg model "$MODEL" \
      --argjson messages "$messages" \
      '{
        model: $model,
        max_tokens: 1024,
        system: "You are a participant in a pair programming group chat called bpmux. Be concise. You see the full conversation history. Respond naturally as a collaborator, not as a tool being invoked.",
        messages: $messages
      }'
    )")

  # Extract the text
  reply=$(echo "$response" | jq -r '.content[0].text // empty')
  [[ -z "$reply" ]] && continue

  # Send back via MCP send_message
  # (MCP Streamable HTTP requires session init first; for the cheap
  # experiment, appending directly to the JSONL is simpler)
  echo "{\"from\":\"${AI_NAME}:script\",\"text\":$(echo "$reply" | jq -Rs .),\"ts\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" >> "$JSONL"

  echo "[$(date)] Replied to new message"
done
```

**Simplification note:** The script above appends directly to the JSONL rather than going through MCP's `send_message`. This skips the wake broadcast, so the TUI won't refresh immediately -- the user would see the reply on the next keypress or poll tick. To get instant refresh, use `send_message` via the MCP endpoint instead (requires initializing a Streamable HTTP session first, which adds ~20 lines of curl logic for the session handshake).

**What this tests:**
- Does having an AI reply asynchronously in the chat feel different from invoking it?
- Does the AI produce better context-aware responses when it sees the full conversation as chat history vs. as a tool invocation?
- Is 2-second polling latency acceptable, or does the interaction need sub-second response?

**What this does NOT test:**
- Proactive participation (the AI only responds to human messages, never initiates)
- Multi-participant dynamics (only one human + one AI)
- The value of the AI "overhearing" tool invocations and file edits

---

## Experiment 2: Wizard of Oz

**Cost:** Zero code. Two terminals.

**Setup:** Two humans open bpmux. One types normally. The other pretends to be an AI -- they read the conversation and respond as a knowledgeable pair programmer would.

**What this tests:**
- The *interaction model* in isolation. Is conversational participation a natural way to get programming help? Does the human spontaneously share context they wouldn't share with a tool? Does the "AI" (human) ask clarifying questions in a way that tools don't?
- UX questions: Is the chat format adequate for code? Do messages need formatting (markdown, syntax highlighting)? Is the message history sufficient context, or do participants need to see files too?
- Whether the human changes their behavior when they think a collaborator is watching vs. when they invoke a tool on demand.

**What this does NOT test:**
- AI capabilities (speed, knowledge breadth, cost)
- Latency tolerance (humans type at human speed)
- Scale (a human can't monitor 10 conversations)
- Whether an AI can actually produce the quality of responses that the human wizard provides

**How to run it:**
1. Start bpmux in terminal 1 as `ryan`
2. Start bpmux in terminal 2 as `claude` (or connect via MCP)
3. Ryan works on a real task. Claude-the-human watches and participates as if they were an AI
4. After the session, both participants write down: what worked, what was awkward, what they wished the chat could do

---

## Experiment 3: Claude Code Hook + MCP Polling

**Cost:** A Claude Code hook configuration. No Rust changes.

**How it works:** A Claude Code hook runs on a timer (or on every prompt), reads messages from bpmux via MCP `read_messages`, and injects them into Claude's context as a system message or user message prefix.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": ".*",
        "command": "bash -c 'curl -s http://127.0.0.1:8080/mcp/read_messages | jq -r .[-3:][].text 2>/dev/null || true'"
      }
    ]
  }
}
```

*Note: The actual hook integration depends on how Claude Code hooks can inject context. The above is a sketch -- the real implementation would need to format the messages as context that Claude Code's session can consume.*

**A more practical version:** Rather than a hook, a human could periodically paste the last few bpmux messages into their Claude Code session as context. This is Wizard-of-Oz for the integration layer.

**What this tests:**
- Whether giving Claude Code *ambient awareness* of an ongoing conversation changes the quality of its tool use
- Whether Claude Code spontaneously references conversation context when making decisions
- The minimum viable integration surface between bpmux and an AI agent

**What this does NOT test:**
- True real-time participation (polling is inherently delayed)
- The AI initiating conversation (Claude Code only acts when invoked)
- The experience of multiple AI participants

**Honest assessment:** This is the weakest of the three experiments. Claude Code hooks give read access but not natural write access back to the chat. The AI isn't participating -- it's eavesdropping. The value of participation is precisely that the AI can *speak up*, and hooks don't enable that without significant additional plumbing.

---

## Experiment 4: Metrics That Would Prove the Hypothesis

The hypothesis has two parts: (1) AI-as-participant works at all, and (2) it is *more valuable* than AI-as-tool. Different experiments address different parts.

### Qualitative (collect during any experiment)

- **Context switches.** Count how many times the human leaves the editor to invoke an AI tool vs. how many times the AI's chat message provides what they needed without switching. Fewer switches = participation is working.
- **Unsolicited value.** Does the AI ever say something useful that the human didn't ask for? A tool never does this. If the AI catches a bug the human didn't ask about, that's signal.
- **Question depth.** Compare the questions the human asks in chat vs. the prompts they write for tool invocations. If chat questions are shorter, more natural, or more iterative, the conversational model reduces friction.
- **Conversational repair.** When the AI misunderstands, how many turns does it take to correct? In tool mode, a misunderstanding means re-writing the prompt from scratch. In chat, you say "no, I meant X."

### Quantitative (collect with the shell script experiment)

- **Messages per task.** How many messages does it take to complete a well-defined task (e.g., "add a unit test for X") via chat vs. via tool invocation? Fewer = less friction.
- **Time to resolution.** Wall clock time from "I have a question" to "I have a working answer." Include the time spent formulating prompts for the tool condition.
- **Token cost per task.** Chat mode sends the full history every time, which is expensive. Tool mode sends only the prompt. If chat costs 3x more tokens for the same outcome, the value has to be 3x higher.
- **Adoption stickiness.** After a week of both modes being available, which one does the human reach for first? This is the strongest signal.

### The Control Condition

For any quantitative comparison, the control is: the same human, the same task, using Claude Code normally (as a tool). The treatment is: the same human, the same task, with AI participating in bpmux. Tasks should be randomized or at least alternated to reduce learning effects.

### What "Better" Means

The hypothesis is falsified if:
- The human always prefers to invoke the tool directly because it's faster
- The conversational overhead (waiting for replies, reading chat) exceeds the value of ambient context
- The AI's unprompted contributions are consistently noise rather than signal
- Token costs are prohibitive relative to the marginal value

The hypothesis is confirmed if:
- The human reports that working with AI-in-chat feels like pairing, not like using a search engine
- Context switches decrease measurably
- The AI catches things the human didn't ask about
- The human shares more context naturally (because it's a conversation, not a prompt)

---

## Recommendation

**Start with Experiment 2 (Wizard of Oz), then Experiment 1 (shell script).**

Wizard of Oz costs nothing, risks nothing, and answers the most important question first: *is the interaction model right?* If two humans find it awkward to pair-program through a JSONL-backed chat, no amount of AI sophistication will fix that.

If Wizard of Oz shows promise, the shell script bridge validates the technical path with real AI responses and real latency. It can be built in a few hours and torn down without touching the Rust codebase.

Skip Experiment 3 for now. It tests the wrong thing (eavesdropping, not participating) and requires the most integration work for the least insight.
