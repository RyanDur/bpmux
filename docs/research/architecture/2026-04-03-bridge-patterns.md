# Bridge Patterns: Connecting AI Tools to bpmux Without Building a Bot

Date: 2026-04-03
Constraint: bpmux stays as communication infrastructure only. No LLM calls inside bpmux. No system prompts. No bot logic.

---

## Context

bpmux already provides two external interfaces:

1. **MCP server** at `/mcp` -- request/response, tool-style (`send_message`, `read_messages`)
2. **WebSocket** at `/ws` -- bidirectional push, JSON protocol with `identify`/`send`/`message` types

Both write to the same JSONL log (`messages-current.jsonl`). Both broadcast through `tokio::sync::broadcast::Sender<Message>`. The question is: what's the thinnest bridge that connects an external AI tool to these interfaces?

---

## 1. The Shell Script Bridge (JSONL Tailing)

**Concept:** A bash script that tails the JSONL log file, detects new messages, calls an external AI API, and writes responses back via the MCP or WebSocket interface.

### Viability

Mechanically possible. The JSONL log at `{session_dir}/messages-current.jsonl` is append-only, so `tail -f` works. Each line is a complete JSON object: `{"from":"ryan:human","text":"hello","ts":"2026-04-02T12:00:00Z"}`. The script parses each new line with `jq`, decides whether to respond, calls an API, and sends the response back.

### Minimal Implementation

```bash
#!/usr/bin/env bash
# bpmux-bridge.sh -- ~25 lines, ships as an example
PORT="${BPMUX_PORT:-4117}"
BOT_NAME="${BOT_NAME:-ai}"
SESSION_DIR=$(readlink ~/.bpmux/sessions/current)
LOG="$SESSION_DIR/messages-current.jsonl"

tail -n0 -f "$LOG" | while IFS= read -r line; do
  from=$(echo "$line" | jq -r '.from')
  text=$(echo "$line" | jq -r '.text')

  # Skip own messages (loop prevention)
  [[ "$from" == *"$BOT_NAME"* ]] && continue

  # Only respond to @mentions
  [[ "$text" != *"@$BOT_NAME"* ]] && continue

  # Call external AI (user provides this command)
  response=$(echo "$text" | $AI_COMMAND)

  # Send back via HTTP POST to MCP, or via websocat to WS
  curl -s -X POST "http://127.0.0.1:$PORT/mcp" \
    -H "Content-Type: application/json" \
    -d "{\"method\":\"tools/call\",\"params\":{\"name\":\"send_message\",\"arguments\":{\"text\":\"$response\"}}}"
done
```

### Failure Modes

1. **Race condition on startup.** `tail -f` starts from the end of the file. If a message arrives between bpmux starting and the script starting, it's missed. Mitigation: `tail -n5 -f` to catch recent messages, with deduplication.

2. **No atomic read.** If `tail` delivers a partial line (write in progress), `jq` fails. This is rare on local filesystems with small writes, but not impossible. Mitigation: validate JSON before processing, skip malformed lines.

3. **Quoting hell.** The `$response` variable needs proper JSON escaping. A shell script that handles newlines, quotes, and Unicode in LLM responses is fragile. This is the biggest practical problem.

4. **No push for WebSocket.** Tailing the log file is poll-adjacent -- it's inotify-based on Linux (real push) but polling on macOS (`tail -f` polls every second). On macOS, response latency is 1-2 seconds.

5. **MCP protocol mismatch.** The MCP endpoint expects the full Streamable HTTP MCP protocol (session initialization, JSON-RPC), not a raw `curl` POST. The script would need to initialize an MCP session first, or use the WebSocket endpoint instead.

### Verdict

Viable as a demo, fragile in practice. The quoting problem alone makes it unsuitable for real LLM responses. Better as a 10-line README example showing the concept than as shipped tooling. The WebSocket approach (pattern 2 or 6) is more robust for actual use.

### Could This Be a 20-Line Script That Ships?

Yes, but only as `examples/bridge.sh` with explicit caveats. It demonstrates the architecture ("bpmux is just a room, the AI connects from outside") without being production tooling. The script should use WebSocket via `websocat` rather than fighting the MCP protocol with `curl`.

---

## 2. The WebSocket-to-stdin Pipe

**Concept:** A bridge reads from bpmux's WebSocket, pipes messages to an AI tool's stdin, reads the tool's stdout, and sends responses back via WebSocket. Unix pipes for AI tools.

### How It Would Work

```
websocat ws://127.0.0.1:4117/ws --> bridge_script --> ai_tool stdin
                                <-- bridge_script <-- ai_tool stdout
```

The bridge handles the WebSocket protocol (identify, parse `message` frames, format `send` frames) and exposes a simpler interface to the AI tool: lines of text in, lines of text out.

### The Problem: bpmux's WebSocket Has a Handshake

bpmux requires an `identify` message before communication begins. The server responds with `welcome`. Then messages flow as JSON envelopes (`{"type":"message","from":"...","text":"...","ts":"..."}`). This is not raw stdin/stdout -- it's a protocol.

A naive pipe like `websocat ws://localhost:4117/ws | ai_tool | websocat ws://localhost:4117/ws` does not work because:
- The first `websocat` needs to send `{"type":"identify","name":"bot"}` before it receives anything useful
- Incoming messages are JSON envelopes, not plain text
- Outgoing messages must be JSON envelopes (`{"type":"send","text":"..."}`)
- The AI tool expects plain text, not JSON protocol frames

### What Would Work

A thin adapter that speaks bpmux's WebSocket protocol on one side and plain text on the other:

```bash
#!/usr/bin/env bash
# ws-stdin-bridge.sh
# Connects to bpmux WS, translates between protocol and plain text

PORT="${BPMUX_PORT:-4117}"
NAME="${BOT_NAME:-ai}"

{
  # Send identify first
  echo '{"type":"identify","name":"'"$NAME"'"}'

  # Read AI tool's stdout, wrap as send messages
  while IFS= read -r line; do
    echo '{"type":"send","text":"'"$(echo "$line" | jq -Rs '.')"'"}'
  done
} | websocat "ws://127.0.0.1:$PORT/ws" | {
  # Parse incoming messages, extract text, pipe to AI tool's stdin
  while IFS= read -r frame; do
    type=$(echo "$frame" | jq -r '.type // empty')
    [ "$type" = "message" ] || continue
    from=$(echo "$frame" | jq -r '.from // empty')
    text=$(echo "$frame" | jq -r '.text // empty')
    echo "$from: $text"
  done
} | $AI_COMMAND
```

### Failure Modes

1. **Bidirectional pipe deadlock.** The above script has a fundamental problem: the `{send}` block reads from `$AI_COMMAND`'s stdout, but `$AI_COMMAND`'s stdin comes from the `{receive}` block, which reads from websocat's output. This creates a circular dependency that requires named pipes (FIFOs) or process substitution to resolve.

2. **Most AI tools are not line-oriented.** Claude CLI (`claude -p`) runs a single prompt and exits. It does not sit in a read-line/write-line loop. Aider's `--message` flag also exits after one response. These tools are request/response, not streaming bidirectional.

3. **No loop prevention.** The bridge would need to filter out messages from itself to avoid infinite loops.

4. **Latency compounding.** Each message goes through: WebSocket frame -> JSON parse -> pipe -> AI tool startup -> API call -> pipe -> JSON format -> WebSocket frame. If the AI tool starts a new process per message, this adds seconds of overhead.

### Verdict

The concept is sound (protocol adapter between WebSocket and plain text), but the implementation is harder than it looks because of bidirectional pipe management and because AI tools are not line-oriented daemons. A purpose-built bridge script (50-80 lines) that handles the protocol, spawns the AI tool per-message, and manages its own WebSocket connection is more practical than trying to compose Unix pipes.

---

## 3. Claude Code as the Bot via CLI

**Concept:** A bridge script watches bpmux for @mentions, invokes `claude -p "respond to: $message"`, and pipes the response back.

### How It Would Work

```bash
# Per-message invocation
while message from bpmux; do
  response=$(claude -p --bare "You are a coding peer. Respond to: $message")
  send_to_bpmux "$response"
done
```

### Strengths

- **Claude Code brings everything.** It has its own tools (file read/write, bash, grep), its own system prompt, its own context management. bpmux provides zero AI logic.
- **`--bare` flag.** Skips hooks, plugins, MCP server discovery. Fast startup for automation.
- **`--output-format json`** gives structured output. `--output-format stream-json` gives streaming events.
- **Codebase awareness.** Claude Code reads CLAUDE.md, understands the project. A bridge that invokes it from the project directory gets project-aware responses for free.

### Implementation

```bash
#!/usr/bin/env bash
# bridge-claude-code.sh
PORT="${BPMUX_PORT:-4117}"
NAME="claude"

# Connect via websocat in background, manage with FIFOs
FIFO_IN=$(mktemp -u)
FIFO_OUT=$(mktemp -u)
mkfifo "$FIFO_IN" "$FIFO_OUT"

# WebSocket connection
(echo '{"type":"identify","name":"'"$NAME"'"}'; cat "$FIFO_IN") \
  | websocat "ws://127.0.0.1:$PORT/ws" > "$FIFO_OUT" &
WS_PID=$!

# Read messages, invoke claude for @mentions
while IFS= read -r frame < "$FIFO_OUT"; do
  type=$(echo "$frame" | jq -r '.type // empty')
  [ "$type" = "message" ] || continue

  from=$(echo "$frame" | jq -r '.from // empty')
  text=$(echo "$frame" | jq -r '.text // empty')

  # Loop prevention
  [[ "$from" == *"$NAME"* ]] && continue

  # Trigger on @mention
  [[ "$text" != *"@$NAME"* ]] && continue

  # Invoke Claude Code
  response=$(claude -p --bare "$text" 2>/dev/null)

  # Send response back
  payload=$(jq -n --arg text "$response" '{"type":"send","text":$text}')
  echo "$payload" > "$FIFO_IN"
done

kill $WS_PID 2>/dev/null
rm -f "$FIFO_IN" "$FIFO_OUT"
```

### Failure Modes

1. **Cold start per invocation.** Each `claude -p` call starts a new process. Even with `--bare`, this takes 1-3 seconds. Conversation feels sluggish if every response requires a fresh process.

2. **No conversation memory.** Each invocation is stateless. Claude Code does not remember previous messages. The bridge would need to pass conversation history as part of the prompt, which means the bridge is doing context management -- the thing we wanted to avoid.

3. **Cost.** Every message triggers a full Claude API call. In a fast-moving conversation, this gets expensive. No batching, no "only respond when it matters" logic unless the bridge adds it.

4. **Shell escaping.** LLM responses with quotes, backticks, and special characters break naive shell scripts. The `jq` approach for JSON construction helps, but the `claude -p "$text"` invocation is fragile if `$text` contains shell metacharacters.

5. **No streaming.** The response arrives all at once after the API call completes. No typewriter effect, no partial responses in the chat.

### Verdict

This is the pattern that most directly achieves "bpmux is just the room, Claude Code is the AI." It works. The cold-start and statelessness problems are real but acceptable for a proof of concept. For a polished experience, the bridge would need to maintain a conversation history and pass it to each invocation -- which means the bridge grows from 20 lines to 50-80 lines.

The Claude Agent SDK is a better fit for a persistent bot (it manages sessions, tools, and context), but it requires Python/TypeScript, not bash. This pattern's value is that it's zero-dependency beyond `claude` CLI and `websocat`.

---

## 4. The MCP Client Bridge

**Concept:** A process that is both an MCP client (connected to bpmux's MCP server) and a pipe to an AI tool. It reads messages from bpmux via MCP, feeds them to the AI, and writes AI responses back via MCP.

### How It Would Work

```
bpmux MCP server <--MCP--> Bridge <--stdin/stdout--> AI tool
```

The bridge calls `read_messages` to get history, detects new messages (by polling or tracking timestamps), sends them to the AI tool, and calls `send_message` with the response.

### The Problem: MCP Is Request/Response

MCP's `read_messages` returns the full message history. There is no push notification when new messages arrive. The bridge must poll:

```python
last_count = 0
while True:
    messages = mcp_client.call_tool("read_messages")
    if len(messages) > last_count:
        new_messages = messages[last_count:]
        for msg in new_messages:
            response = ai_tool.process(msg)
            mcp_client.call_tool("send_message", text=response)
        last_count = len(messages)
    sleep(1)
```

This works but adds 0-1 second latency per message and generates constant read traffic.

### When This Pattern Makes Sense

If the AI tool is already an MCP client. Claude Code, Cursor, Windsurf -- these tools already speak MCP. If someone configures bpmux as an MCP server in their Claude Code settings, Claude Code can call `read_messages` and `send_message` directly. No bridge needed. The AI tool IS the bridge.

This is the pattern that's already working today: someone adds bpmux's MCP server to their `claude_desktop_config.json` or their `.mcp.json`, and their AI tool can send/read messages. The "bridge" is just MCP server configuration.

### The Dedicated Bridge Variant

A standalone process that acts as an MCP client to bpmux and an MCP server to an AI tool. This is double-MCP: the bridge speaks MCP in both directions.

This is overengineered for the use case. If the goal is "pipe messages between bpmux and an AI tool," WebSocket is simpler than MCP-to-MCP. MCP's value is as an integration point for tools that already speak it, not as a general-purpose message bus.

### Verdict

The MCP client bridge is not a pattern to build -- it's a pattern that already exists. Any MCP-aware AI tool (Claude Code, Cursor) can already connect to bpmux's MCP server. The "bridge" is zero code: it's a configuration line pointing the AI tool at `http://127.0.0.1:4117/mcp`.

For a dedicated bot process, the WebSocket endpoint is the right choice (push-based, bidirectional). MCP is the right choice for integrating AI tools that already have their own UX and decide when to call tools.

---

## 5. OpenCode/Aider as Backend

**Concept:** Use existing AI coding tools as the AI participant, with bpmux routing messages to/from them.

### Aider

Aider has `--message` / `-m` flag for non-interactive use:

```bash
aider --message "Review this approach: $text" --no-auto-commits
```

This runs, applies any edits to files, prints the response, and exits. The response goes to stdout.

**Bridge pattern:**
```bash
# Watch bpmux, invoke aider per message
response=$(aider -m "$text" --no-auto-commits 2>&1)
send_to_bpmux "$response"
```

**Strengths:**
- Aider can edit files. A bpmux message like "@aider add error handling to src/main.rs" would actually modify code.
- Aider manages its own git context, file selection, and edit format.

**Problems:**
- Aider is designed for a single human operator, not for receiving messages from a chat room. It assumes it owns the terminal.
- `--message` mode exits after each invocation. No persistent session, no memory of previous messages.
- Aider's output is formatted for terminal display (colors, diffs, status messages), not for chat. The bridge would need to strip ANSI codes and extract the relevant response text.
- Aider starts slowly (loads repo map, model config). 3-5 seconds per invocation.

### OpenCode

OpenCode has `-p` flag for non-interactive use:

```bash
opencode -p "Review this approach: $text"
```

Similar to Claude Code's `-p` flag. Runs the prompt, prints the response, exits.

**Strengths:**
- Provider-agnostic: supports 75+ models. The user picks the model, bpmux doesn't care.
- Has tool use (file read/write, bash execution) in non-interactive mode.
- Auto-approves all permissions in non-interactive mode.

**Problems:**
- Same statelessness issue as all per-invocation patterns: no conversation memory.
- Same cold-start issue: 2-4 seconds per invocation.
- Output formatting designed for terminal, not chat.

### The Deeper Issue

All these tools (Claude Code, Aider, OpenCode) are designed as interactive terminals for a single human. Using them as chat participants via their non-interactive flags is fighting against their design. They work, but awkwardly:

- No conversation state between invocations
- Cold start on every message
- Terminal-formatted output needs cleanup
- They assume file system access and git context that may not be relevant in a chat

### Verdict

Viable for proof-of-concept demos. A bridge that invokes `aider -m "$message"` or `opencode -p "$message"` per @mention works and shows that bpmux can integrate with any AI tool. But the experience is mediocre due to statelessness and cold start.

The better pattern is for these tools to connect to bpmux via MCP (they all support MCP servers) rather than being invoked as subprocesses. That way the tool maintains its own session state and responds when it decides to, not when the bridge polls.

---

## 6. The "bpmux is just netcat for AI" Framing

**Concept:** bpmux provides the WebSocket, the user pipes whatever they want through it.

```bash
websocat ws://localhost:4117/ws | my_ai_tool | websocat ws://localhost:4117/ws
```

### What Breaks

Everything. This command:
1. Opens two separate WebSocket connections (two `websocat` instances), which means two separate identities in bpmux
2. The first `websocat` never sends `identify`, so bpmux rejects it after 5 seconds
3. `my_ai_tool` receives raw JSON protocol frames, not plain text
4. `my_ai_tool`'s output goes to the second `websocat` as raw text, not as `{"type":"send","text":"..."}` JSON envelopes
5. The pipe is unidirectional twice, not bidirectional once

### Could websocat Do This Properly?

websocat supports complex modes. A closer attempt:

```bash
websocat -t ws://localhost:4117/ws \
  --exec 'my_ai_tool' \
  --init-text '{"type":"identify","name":"bot"}'
```

But websocat's `--exec` mode spawns a new process per connection and pipes WebSocket frames to/from it. The process receives raw JSON frames and must output raw JSON frames. This means `my_ai_tool` must understand bpmux's protocol -- at which point it's a bot, not a generic tool being piped.

### websocketd (Reverse Direction)

websocketd turns any stdin/stdout program into a WebSocket server. This is the opposite direction: the AI tool becomes a server, and bpmux would need to connect to it as a client. That inverts the architecture -- bpmux would need to be an MCP/WS client to external processes, which violates the "bpmux is just infrastructure" constraint.

### The Honest Framing

bpmux is not netcat. Netcat has no protocol -- it's raw bytes. bpmux has a protocol (identify, JSON envelopes, message types). This protocol exists for good reasons (identity, loop prevention, message attribution), but it means you cannot blindly pipe to/from bpmux.

The better framing: **bpmux is IRC for AI.** IRC has a protocol too. You cannot pipe raw text to an IRC server. But the protocol is simple enough that a 30-line script can speak it. bpmux's WebSocket protocol is similarly simple: send one JSON line to identify, then send/receive JSON envelopes. A bridge script that speaks this protocol is 30-50 lines, not a framework.

### What Would Make the Netcat Framing Work

If bpmux added a **raw text mode** on a separate endpoint (e.g., `/ws/raw`):
- No identify required (identity assigned by connection order or query param: `/ws/raw?name=bot`)
- Incoming messages are plain text lines: `ryan: hello`
- Outgoing messages are plain text lines (anything the client sends becomes a message)

This would be a genuinely pipe-friendly interface. The cost is losing structured data (no JSON, no timestamps in the stream, no message types). For quick experiments and demos, that trade-off might be worth it.

### Verdict

The "netcat for AI" framing does not work with the current WebSocket protocol. The protocol is intentionally structured. A thin adapter script (pattern 2 or 3) is needed between the structured protocol and a plain-text tool. A raw text endpoint could make the framing work but trades structure for simplicity.

---

## 7. Configuration Over Code

**Concept:** Instead of building a bot binary, bpmux has a config option that spawns an external process and pipes messages to/from it.

```toml
# bpmux.toml
[bridge]
command = "claude -p --bare"
name = "claude"
trigger = "@claude"
```

bpmux spawns the command, pipes relevant messages to its stdin, reads responses from its stdout, and sends them back to the session.

### How It Would Work

On startup, bpmux reads the config, spawns the process, and manages the bridge internally:

```
bpmux (internal) --> stdin of `claude -p --bare` --> reads stdout --> sends as message
```

This is LSP-style stdio transport: the spawned process communicates via stdin/stdout with a defined protocol (or plain text).

### Strengths

- **Zero external tooling.** The user configures a command in a TOML file, starts bpmux, and the AI is there. No second terminal, no separate script.
- **bpmux manages the lifecycle.** Process crashes? Restart it. bpmux shuts down? Kill the child. No orphan processes.
- **Trigger logic in bpmux.** The `trigger = "@claude"` config means bpmux filters messages before sending them to the subprocess. The subprocess only sees relevant messages.

### Problems

1. **bpmux now has bot logic.** The trigger filtering, message formatting for stdin, response parsing from stdout -- this is bridge code inside bpmux. It's not LLM code, but it's bot orchestration code. This pushes bpmux from "communication infrastructure" toward "bot framework."

2. **What protocol on stdin/stdout?** Plain text? JSON lines? If the subprocess is `claude -p`, it expects a single prompt and exits. It's not a daemon that reads lines. The config-driven bridge needs to either:
   - Spawn a new process per message (expensive, stateless)
   - Keep a long-running process and define a stdin/stdout protocol (now bpmux defines a protocol for AI tools)

3. **The config grows.** Once you have `command`, `name`, and `trigger`, users want: `system_prompt`, `max_tokens`, `context_window`, `rate_limit`, `filter_own_messages`, `response_format`. The config becomes a bot specification language. bpmux is building a bot framework in TOML.

4. **Testing complexity.** bpmux's test suite now needs to test process spawning, stdin/stdout piping, error handling for crashed subprocesses, and config parsing. These tests have nothing to do with communication infrastructure.

### What Keeps It Thin

If the config is minimal and the protocol is plain text:

```toml
[bridge]
command = "./my-bridge.sh"
name = "ai"
```

bpmux spawns the command, sends each new message as a line of text to stdin (`ryan: hello`), and treats each line from stdout as a response to send. No trigger logic (the script decides). No response formatting (the script handles it). No restart logic (if it dies, it dies).

This is closer to inetd or websocketd: bpmux connects a subprocess to the message stream, and the subprocess does whatever it wants.

### Verdict

The thin version (spawn command, pipe plain text lines, no trigger logic) is viable and stays close to "communication infrastructure." The moment bpmux adds trigger filtering, system prompt config, or restart logic, it's building a bot framework.

The thin version is essentially what websocketd does: connect a subprocess to a bidirectional stream. The question is whether this should be in bpmux core or in an external tool. If it's 30 lines of code in bpmux, it might be worth the convenience. If it grows beyond that, it should be external.

---

## 8. What's the Minimum bpmux Needs to Provide?

### What Already Exists

bpmux already provides everything needed for external bridges:

| Interface | Status | Purpose |
|---|---|---|
| WebSocket at `/ws` | Implemented | Bidirectional push, JSON protocol, identify/send/message |
| MCP at `/mcp` | Implemented | Tool-style integration (send_message, read_messages) |
| JSONL log | Implemented | Append-only file, tail-able |
| Broadcast channel | Implemented | Internal fan-out to all consumers |

### What's Missing

**Nothing critical.** All eight bridge patterns in this document can be built with the existing interfaces. The WebSocket endpoint is the most capable -- it supports push, bidirectional communication, and real-time message delivery.

### Ranked by Impact

What bpmux could add, in order of value:

1. **An example bridge script** (high value, zero code in bpmux). A `examples/bridge-claude.sh` that demonstrates connecting Claude Code to bpmux via WebSocket. 30-50 lines of bash using `websocat` and `jq`. Shows the architecture without bpmux owning any AI logic. This is documentation, not a feature.

2. **A raw text WebSocket endpoint** (medium value, ~40 lines in bpmux). `/ws/raw?name=bot` that strips the protocol: no identify required, plain text in/out. Makes the "netcat for AI" framing work. Lowers the barrier for quick experiments.

3. **A config-driven process spawner** (medium value, ~60 lines in bpmux). The thin version from pattern 7: spawn a command from config, pipe plain text lines. No trigger logic, no restart, no protocol. Convenience for users who want a single-command experience.

4. **History endpoint** (low value, existing via MCP). A simple HTTP GET `/messages` that returns the JSONL content. Easier for scripts than speaking the MCP protocol. Could be one route in axum: `async fn get_messages() -> Json<Vec<Message>>`.

### The Thinnest Layer

The absolute minimum: **just the WebSocket endpoint** (which already exists). Every bridge pattern in this document works with it. The example script is documentation, not code. The raw text endpoint and process spawner are conveniences, not necessities.

bpmux's job is done. It provides the room. External tools provide the AI. The bridge between them is 30-50 lines of bash, or a few lines of Python, or an MCP configuration entry. None of that code belongs in bpmux.

---

## Pattern Comparison

| Pattern | Lines of bridge code | Latency | Statefulness | Dependencies | Production-ready? |
|---|---|---|---|---|---|
| 1. Shell/JSONL tail | ~25 | 1-2s (macOS poll) | Stateless | jq, curl/websocat | No (quoting fragility) |
| 2. WS-to-stdin pipe | ~50-80 | <1s | Depends on tool | websocat, jq | Marginal (pipe complexity) |
| 3. Claude Code CLI | ~40-60 | 2-4s (cold start) | Stateless per call | claude CLI, websocat | Yes (for demos) |
| 4. MCP client bridge | 0 (config only) | 0-1s (polling) | Tool manages it | MCP-aware AI tool | Yes (already works) |
| 5. Aider/OpenCode | ~40-60 | 3-5s (cold start) | Stateless per call | aider/opencode, websocat | Yes (for demos) |
| 6. Netcat piping | N/A | N/A | N/A | N/A | No (protocol mismatch) |
| 7. Config spawner | 0 (config only) | <1s | Depends on tool | None (in bpmux) | Possible (scope risk) |
| 8. Just the WebSocket | 0 | <1s | N/A | N/A | Yes (already exists) |

---

## Recommendation

**Ship an example, not a feature.**

The WebSocket endpoint already exists and works. The MCP endpoint already exists and works. Every AI tool that matters (Claude Code, Cursor, Aider, OpenCode) either speaks MCP or can be invoked via CLI. The bridge is 30-50 lines of bash.

Concretely:
1. Write `examples/bridge-claude.sh` -- a working bridge that connects Claude Code CLI to bpmux via WebSocket. Include it in the repo. This is the proof that bpmux is infrastructure, not a framework.
2. Consider a raw text endpoint (`/ws/raw`) as a future convenience, but do not block on it.
3. Do not build a config-driven process spawner in bpmux core. That path leads to bot framework scope creep.
4. Do not build MCP-to-MCP bridges. The tools that speak MCP already connect directly.

The acceptance test (`tests/story3_acceptance_test.rs`) already demonstrates the full pattern: a WebSocket client connects, identifies as "claude", receives messages, calls the Claude API, and sends responses back. That test IS the bridge. The example script is just that test translated to bash.

---

## Sources

- bpmux WebSocket implementation: `src/mcp/ws.rs`, `src/mcp/protocol.rs`
- bpmux MCP server: `src/mcp/server.rs`
- bpmux session log: `src/session/log.rs`
- Prior research: `docs/research/architecture/2026-04-02-external-peer-model.md`, `docs/research/architecture/2026-04-02-websocket-protocol.md`
- Story 3 acceptance test: `tests/story3_acceptance_test.rs`
- websocat: https://github.com/vi/websocat
- websocketd: https://github.com/joewalnes/websocketd
- Claude Code CLI: https://code.claude.com/docs/en/cli-reference
- Claude Code headless mode: https://code.claude.com/docs/en/headless
- Aider scripting: https://aider.chat/docs/scripting.html
- OpenCode CLI: https://opencode.ai/docs/cli/
