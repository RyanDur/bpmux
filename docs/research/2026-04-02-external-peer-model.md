# External Peer Model: AI as IRC Bot, Not Embedded Framework

Date: 2026-04-02
Prompted by: Design critique (section 1) — bpmux should be the chat room, the AI should connect as an external peer

---

## The Core Question

The design critique's strongest argument: embedding LLM client code inside bpmux turns a chat tool into an AI framework. The alternative is the IRC bot model — bpmux exposes a bidirectional stream interface, and an AI process connects as a peer. bpmux never knows or cares that the peer is an AI.

This document investigates what that would actually look like, mechanically.

---

## 1. What Bidirectional Protocol Does bpmux Expose for Peers?

MCP (the current external interface) is request/response. The AI must poll `read_messages` to see new messages. It cannot receive a push notification when someone speaks. This is the wrong shape for a conversation participant.

The right shape is a **bidirectional message stream**: the peer connects once, receives every new message as it arrives (push), and can send messages at any time. Three viable options:

### Option A: Local WebSocket (recommended)

A WebSocket endpoint on the existing axum server. The peer connects, upgrades to WebSocket, and gets a full-duplex channel. Messages flow both directions as JSON frames.

**Why this wins:**
- axum 0.8 has built-in WebSocket support via `axum::extract::ws::WebSocketUpgrade` — no new dependencies needed
- WebSocket is a well-understood protocol with client libraries in every language
- Full-duplex: the server pushes new messages to the client, the client sends messages back, simultaneously
- Works over `127.0.0.1` for local bots or over a network for remote ones
- The existing axum server on port 4117 can host both the MCP endpoint (`/mcp`) and a WebSocket endpoint (`/ws`) on the same port

**Wire format:** Each WebSocket frame is a JSON object. Two message types:

```
Server -> Client (new message appeared in the session):
{"type": "message", "data": {"from": "ryan:human", "text": "what do you think?", "ts": "2026-04-02T12:00:00Z"}}

Client -> Server (bot sends a message):
{"type": "send", "text": "I think the approach looks solid because..."}
```

Optionally, on connect, the server sends a backfill of recent history so the bot has context:

```
{"type": "history", "messages": [...]}
```

### Option B: Unix Domain Socket with Newline-Delimited JSON

A Unix socket at a known path (e.g., `~/.bpmux/sessions/current/peer.sock`). The bot connects and reads/writes newline-delimited JSON (NDJSON). Each line is a complete JSON message.

**Pros:** No HTTP overhead, slightly simpler than WebSocket, natural for local-only bots.
**Cons:** Unix-only (no Windows), no existing ecosystem of client libraries, requires the bot to know the socket path, cannot work remotely.

### Option C: stdin/stdout of a Child Process

bpmux spawns the bot as a child process and pipes messages through stdin/stdout (NDJSON). This is how Language Server Protocol (LSP) transports work.

**Pros:** Extremely simple. No networking. The bot is just a program that reads stdin and writes stdout.
**Cons:** bpmux must manage the child process lifecycle. Cannot support bots that start independently. Cannot support remote bots. Only one bot per spawned process.

### Recommendation

**WebSocket on the existing axum server.** It requires zero new dependencies (axum already supports it), works for local and remote bots, supports multiple simultaneous connections, and uses a protocol that every language has a client library for. Unix sockets and stdin/stdout can be added later as convenience layers on top of the same internal broadcast mechanism.

---

## 2. How Did IRC Bots Connect?

An IRC bot is a regular IRC client. It opens a TCP socket to the IRC server, sends the `NICK` and `USER` registration commands, joins a channel with `JOIN #channel`, and then enters a read loop:

```
connect(server:6667)
send("NICK mybot")
send("USER mybot 0 * :My Bot")
send("JOIN #channel")
loop {
    line = read_line()
    if line starts with "PING" { send("PONG ...") }
    if line contains "PRIVMSG #channel" {
        parse the message
        if should_respond(message) {
            send("PRIVMSG #channel :my response")
        }
    }
}
```

Key properties of this model:

1. **The bot is just a client.** The IRC server does not know it is a bot. It treats it identically to a human client.
2. **Push, not poll.** The server sends every channel message to the bot in real time. The bot does not ask for messages — they arrive.
3. **Same protocol for read and write.** The bot reads `PRIVMSG` lines and sends `PRIVMSG` lines. Symmetric.
4. **The bot decides when to speak.** The server broadcasts everything. The bot's trigger logic (respond to `!command`, respond when mentioned, respond to every message) is entirely the bot's concern. The server has no opinion.
5. **Multiple bots can join the same channel.** Each is an independent client connection.

bpmux's WebSocket endpoint would replicate this model exactly. The WebSocket connection is the TCP socket. JSON messages are the IRC protocol lines. The bot reads messages, decides whether to respond, and sends responses back. bpmux broadcasts to all connected WebSocket clients, just as an IRC server broadcasts to all channel members.

---

## 3. The WebSocket Architecture

### How It Fits the Existing Codebase

bpmux already has:
- An axum server on port 4117 serving the MCP endpoint at `/mcp`
- A `broadcast::channel` (`wake_tx`/`wake_rx`) for notifying the TUI when MCP writes a message
- `session::append_message` and `session::read_messages` for the JSONL log

The WebSocket endpoint extends this cleanly:

```
axum::Router::new()
    .nest_service("/mcp", mcp_service)       // existing
    .route("/ws", get(ws_handler))            // new
```

### Internal Broadcast Mechanism

The current `broadcast::channel<()>` sends a wake signal but no payload. To support WebSocket push, this changes to broadcast the actual message:

```
broadcast::channel<Message>(64)
```

When any source writes a message (TUI, MCP, WebSocket client), it:
1. Appends to the JSONL log (source of truth)
2. Sends the message on the broadcast channel

All consumers (TUI wake, WebSocket clients) receive the message from the broadcast channel. The TUI uses it to update its display. Each WebSocket connection forwards it as a JSON frame to the connected bot.

### Connection Lifecycle

```
Bot connects to ws://127.0.0.1:4117/ws
  |
  v
Server sends: {"type": "history", "messages": [...last N messages...]}
  |
  v
Server subscribes the connection to the broadcast channel
  |
  v
Two concurrent loops:
  - Read from broadcast -> forward to WebSocket client (push)
  - Read from WebSocket client -> append_message + broadcast (receive)
  |
  v
Bot disconnects: connection is dropped, subscription is cleaned up
```

### What This Looks Like in axum

```rust
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(socket: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let mut broadcast_rx = state.msg_tx.subscribe();

    // Send history backfill
    // ...

    // Forward broadcast messages to WebSocket client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap_or_default();
            if ws_tx.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Receive messages from WebSocket client
    while let Some(Ok(msg)) = ws_rx.next().await {
        if let Message::Text(text) = msg {
            // parse, create Message, append_message, broadcast
        }
    }

    send_task.abort();
}
```

This is roughly 50-80 lines of Rust. It uses `axum::extract::ws` which is already available — no new crate needed.

---

## 4. What Does This Look Like for the User?

### Scenario: Developer starts bpmux, then starts an AI bot

```bash
# Terminal 1: Start bpmux
$ bpmux create --name ryan

# Terminal 2: Start the AI bot (separate process)
$ bpmux-claude-bot --url ws://127.0.0.1:4117/ws --name claude

# Or with a generic bot launcher:
$ bpmux-bot --provider anthropic --model claude-sonnet-4-20250514 --url ws://127.0.0.1:4117/ws
```

The bot connects. In the TUI, a system message appears: "claude joined the session." Now the bot receives every message in real time. When someone types `@claude what do you think?`, the bot's trigger logic fires, it calls the Anthropic API, and sends the response back over the WebSocket. The response appears in bpmux like any other message.

### Scenario: Bundled convenience

bpmux could ship a `--with-bot` flag that spawns the bot as a child process:

```bash
$ bpmux create --name ryan --with-bot claude
```

This is sugar — under the hood it starts bpmux, then spawns a bot process that connects via WebSocket. The user gets a single command, but the architecture stays clean.

### Scenario: Multiple bots

```bash
# Terminal 1: bpmux
$ bpmux create --name ryan

# Terminal 2: Claude bot
$ bpmux-bot --provider anthropic --model claude-sonnet-4-20250514 --url ws://127.0.0.1:4117/ws --name claude

# Terminal 3: Local Ollama bot
$ bpmux-bot --provider ollama --model llama3.1 --url ws://127.0.0.1:4117/ws --name llama
```

Both bots are in the session. They each see all messages, including each other's. The trigger logic in each bot determines when and whether to respond.

---

## 5. Comparison: External Peer vs. Direct API Calls Inside bpmux

### What You Gain with the External Peer Model

**bpmux stays a communication tool, not an AI framework.**
- No `src/ai/` bounded context. No provider trait. No Anthropic client. No token counting. No context window management. No system prompt configuration. bpmux's codebase stays focused on messaging.
- The JSONL log, the TUI, the MCP server, and the WebSocket endpoint — all are communication infrastructure. None of them know about LLMs.

**The bot can be written in any language.**
- A Python bot using the official Anthropic SDK. A TypeScript bot using the Agent SDK. A Rust bot using `reqwest`. A bash script using `curl` and `websocat`. The protocol is a WebSocket with JSON — universal.

**The bot evolves independently.**
- When Anthropic changes their API, the bot updates. bpmux does not. When a new provider appears, someone writes a new bot. bpmux does not change.
- Different teams can use different bots with different behaviors, trigger policies, and system prompts. bpmux has no opinion.

**Multiple bots with zero architectural complexity.**
- Each bot is just another WebSocket connection. bpmux broadcasts to all of them. No special multi-agent orchestration code.

**Testing is simpler.**
- bpmux tests: does the WebSocket endpoint correctly broadcast messages? Does it accept messages from clients? These are pure communication tests.
- Bot tests: does the bot correctly call the LLM API? Does its trigger logic work? These are isolated to the bot.
- No integration tests that require mocking LLM API calls inside bpmux.

**Separation of concerns matches the design spec.**
- The v1 design spec says bpmux is tool-agnostic and that AI tools connect via MCP. The external peer model extends this principle — AI participants also connect via a clean protocol, not via embedded code.

### What You Lose

**No single-binary experience out of the box.**
- The user must start bpmux AND start a bot. Two processes, two terminals (unless `--with-bot` sugar is provided).
- The "direct API calls" approach gives a single `bpmux create --name ryan` command with AI participation built in.

**bpmux cannot control the AI's behavior.**
- With embedded AI, bpmux owns the system prompt, the trigger policy, and the context window. It can enforce behavior (e.g., concise responses, never respond to its own messages).
- With an external bot, the bot controls all of this. bpmux cannot prevent a badly written bot from spamming the channel or creating infinite loops.
- Mitigation: bpmux can enforce rate limits on WebSocket message sends. It can also include a `from` field requirement and reject messages where a peer tries to impersonate another peer.

**The bot must manage its own context window.**
- With embedded AI, bpmux assembles the message history and handles token counting centrally.
- With an external bot, the bot receives the message stream and must maintain its own sliding window. This is duplicated work if multiple bot implementations exist.
- Mitigation: a shared library (Rust crate or npm package) can provide common bot infrastructure — context windowing, trigger logic, history management. The bpmux project could publish `bpmux-bot-sdk` alongside the main binary.

**Latency for the history backfill.**
- On connect, the bot needs recent history to have context. If the session has thousands of messages, sending them all over the WebSocket is slow.
- Mitigation: send only the last N messages (configurable). The bot can also read the JSONL log file directly if it is on the same machine.

### Net Assessment

The external peer model trades "batteries included" UX for architectural cleanliness, extensibility, and separation of concerns. The losses (two-process startup, bot manages its own context) are real but mitigable. The gains (bpmux stays simple, any language, multiple bots, independent evolution) compound over time.

The design critique is right: if bpmux embeds LLM client code, it takes on the maintenance burden of every LLM API change, every provider quirk, and every context management edge case. That is a large surface area for a v1 chat tool to own.

---

## 6. Could This Support Multiple Bots?

Yes, trivially. Each bot is an independent WebSocket connection. bpmux's broadcast channel sends every message to every subscriber. Multiple bots connecting simultaneously requires zero additional code beyond the basic WebSocket handler.

Considerations for multiple bots:

- **Loop prevention.** Bot A speaks, Bot B sees it and responds, Bot A sees the response and responds, ad infinitum. Each bot must implement its own loop prevention (e.g., never respond to messages from peers with a bot identifier, or use a cooldown timer). bpmux can help by including the sender's identity in each message, and optionally by enforcing a per-sender rate limit.
- **Turn-taking.** With multiple bots, who speaks first? This is the bot's responsibility, not bpmux's. Bots can use conventions like a short random delay before responding, or only responding when explicitly mentioned by name.
- **Context divergence.** Each bot has its own context window and may interpret the conversation differently. This is a feature, not a bug — it is how human participants work too.

Example: Claude bot (frontier model, great at architecture) and a local Ollama bot (fast, private, good for quick lookups). The user addresses them by name: `@claude review this design` or `@llama what is the syntax for...`.

---

## 7. Does axum's Existing WebSocket Support Work?

Yes. axum 0.8 (already in Cargo.toml) includes WebSocket support via `axum::extract::ws`. The key types:

- `WebSocketUpgrade` — extractor that upgrades an HTTP request to a WebSocket connection
- `WebSocket` — the connected socket, which implements `Stream + Sink` for reading and writing messages
- `Message` — enum with `Text(String)`, `Binary(Vec<u8>)`, `Ping`, `Pong`, `Close`

The existing axum router at port 4117 can add a WebSocket route alongside the MCP service:

```rust
let router = axum::Router::new()
    .nest_service("/mcp", mcp_service)
    .route("/ws", get(ws_handler));
```

No new dependencies are needed. The `futures` crate (already in Cargo.toml) provides `StreamExt` and `SinkExt` for splitting the WebSocket into read/write halves. The `tokio::sync::broadcast` channel (already used for wake signals) provides the pub/sub fan-out to all connected WebSocket clients.

The only change to the existing broadcast channel: it currently sends `()` (a wake signal with no payload). It would need to send the actual `Message` so WebSocket clients receive the message content, not just a wake notification. This is a one-line type change: `broadcast::channel::<Message>(64)` instead of `broadcast::channel::<()>(16)`.

---

## 8. Implementation Sketch

### What Changes in bpmux

1. **Broadcast channel carries messages, not wake signals.** `broadcast::Sender<()>` becomes `broadcast::Sender<Message>`. The TUI wake logic changes from `wake_rx.try_recv().is_ok()` to receiving the actual message.

2. **New WebSocket handler.** A `ws_handler` function in the MCP server module (or a new `src/peer/` module) that accepts WebSocket upgrades, subscribes to the broadcast channel, and runs the bidirectional forwarding loop.

3. **Route added to axum router.** One new `.route("/ws", get(ws_handler))` line.

4. **Shared state.** The axum router needs access to the broadcast sender, the session directory, and possibly a peer registry (for tracking connected bots). This is a small `AppState` struct passed via axum's `State` extractor.

Estimated scope: 80-120 lines of new Rust code, plus minor refactoring of the broadcast channel type.

### What Exists Separately (the Bot)

A separate binary (could live in the same repo as a workspace member, or in a separate repo):

1. Connect to `ws://127.0.0.1:4117/ws`
2. Receive history backfill
3. Enter a loop: receive messages, apply trigger logic, call LLM API, send responses
4. Manage its own context window (sliding window of recent messages)

This is where all the LLM complexity lives: provider abstraction, token counting, streaming, system prompts, trigger policies. None of it is in bpmux.

---

## 9. Relationship to MCP

The WebSocket peer protocol and MCP serve different purposes and coexist:

| | MCP (`/mcp`) | WebSocket (`/ws`) |
|---|---|---|
| Direction | Request/response (client-initiated) | Bidirectional (push + send) |
| Use case | AI tool integration (Claude Code, Cursor) | Real-time participant (bot in the conversation) |
| Who connects | The user's existing AI coding tool | A dedicated bot process |
| Message flow | Bot polls `read_messages`, calls `send_message` | Bot receives push, sends messages directly |
| Trigger logic | In the AI tool (Claude Code decides when to call the MCP tool) | In the bot (bot decides when to respond) |
| Latency | Polling interval (seconds) | Real-time (milliseconds) |

MCP is the right interface for AI tools that have their own UX and workflows (like Claude Code). WebSocket is the right interface for dedicated bot participants that exist solely to be in the bpmux conversation.

Both write to the same JSONL log. Both appear in the TUI. Both are visible to all participants. The difference is the connection model, not the message model.

---

## 10. Summary

| Question | Answer |
|---|---|
| What protocol for bidirectional peer connections? | WebSocket on the existing axum server (port 4117, `/ws` endpoint) |
| New dependencies needed? | None — axum 0.8 + futures + tokio broadcast already in Cargo.toml |
| How did IRC bots work? | TCP connect, receive push messages, send when triggered — exactly the WebSocket model |
| Can the AI be a separate process? | Yes — connects via WebSocket, receives all messages, sends responses back |
| What does it look like for the user? | Start bpmux, start bot process (or use `--with-bot` sugar). Bot appears in the conversation. |
| What do you gain vs. embedded AI? | bpmux stays simple, any-language bots, independent evolution, multiple bots trivially, cleaner testing |
| What do you lose vs. embedded AI? | Two-process startup, bot manages its own context, bpmux cannot enforce AI behavior |
| Multiple bots? | Yes, trivially — each bot is an independent WebSocket connection |
| Implementation scope in bpmux? | ~80-120 lines: broadcast channel type change, WebSocket handler, route registration |
| Does this replace MCP? | No — MCP stays for AI tool integration, WebSocket adds real-time peer participation |
