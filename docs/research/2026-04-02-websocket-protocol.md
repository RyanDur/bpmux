# WebSocket Protocol Specification for bpmux

Date: 2026-04-02
Status: Draft
Input: external-peer-model research, design-reconciliation, signaling-patterns, story-candidates (Stories 3 and 5), existing codebase analysis, Discord/Slack/Matrix protocol patterns

---

## Purpose

This document specifies the WebSocket protocol that bpmux exposes for real-time bidirectional participation. Any client -- human TUI, AI bot, Web UI, test harness -- connects to this endpoint and participates in the session as a peer. bpmux does not distinguish between client types at the protocol level.

The WebSocket endpoint is complementary to MCP. MCP (`/mcp`) serves AI tool integration (request/response, tool-style). WebSocket (`/ws`) serves real-time participation (bidirectional, push-based). Both write to the same JSONL log. Both appear in the TUI. Both are visible to all participants.

---

## 1. Connection

### Endpoint

```
ws://127.0.0.1:{port}/ws
```

The WebSocket endpoint lives on the same axum server as the MCP endpoint (default port 4117). The axum router hosts both:

```rust
let router = axum::Router::new()
    .nest_service("/mcp", mcp_service)
    .route("/ws", get(ws_handler))
    .with_state(app_state);
```

### Upgrade

Standard HTTP upgrade. The client sends an HTTP GET with `Upgrade: websocket` headers. axum handles the upgrade via `axum::extract::ws::WebSocketUpgrade`. No subprotocol negotiation is required.

### Authentication (v1)

None. The server binds to `127.0.0.1` only. Any local process can connect. This matches the MCP endpoint's current security model.

Future versions (Story 6) add session token authentication: the client sends a token in the `Sec-WebSocket-Protocol` header or as a query parameter (`?token=...`), and the server validates it before completing the upgrade. Unauthenticated connections are rejected with HTTP 401.

### Connection Lifecycle

```
Client opens WebSocket to ws://127.0.0.1:4117/ws
  |
  v
Client sends: identify message (name + client type)
  |
  v
Server responds: welcome message (session_id, your assigned identity)
  |
  v
Server sends: history backfill (last N messages)
  |
  v
Bidirectional message flow begins
  - Server pushes new messages to client
  - Client sends messages to server
  |
  v
Client disconnects (or server shuts down)
  - Server broadcasts disconnect event to remaining clients
  - No cleanup required -- broadcast subscription drops automatically
```

---

## 2. Message Format

All WebSocket frames are JSON text. Binary frames are not used. Each frame contains exactly one message envelope.

### Envelope Structure

Every message has a `type` field that discriminates the payload. This follows the tagged-union pattern used by Discord (opcode-based), Slack (type field), and Matrix (event type). bpmux uses a string `type` for readability over numeric opcodes.

```json
{
  "type": "...",
  "...payload fields..."
}
```

### Message Types: Client to Server

#### `identify`

Sent immediately after connection. Required before any other message. The server rejects frames from unidentified clients.

```json
{
  "type": "identify",
  "name": "claude",
  "client_type": "bot"
}
```

Fields:
- `name` (string, required): Display name for this participant. Appears in `from` fields of messages attributed to this client. Max 64 characters, alphanumeric plus hyphens and underscores.
- `client_type` (string, required): One of `"human"`, `"bot"`, `"web-ui"`. Determines the suffix in the `from` field (e.g., `claude:bot`, `ryan:web-ui`). The server uses this for display purposes only -- it does not gate behavior on client type.

The server responds with `welcome` or `error`.

#### `send`

Send a chat message to the session.

```json
{
  "type": "send",
  "text": "I think the approach looks solid because..."
}
```

Fields:
- `text` (string, required): The message content. Max 4096 bytes UTF-8. The server rejects messages that exceed this limit.

The server assigns `from` and `ts` -- the client cannot spoof sender identity. This prevents impersonation. The resulting `Message` is appended to the JSONL log and broadcast to all clients (including the sender, as confirmation).

#### `typing`

Indicates the client is composing a message. Optional -- clients may choose not to send these.

```json
{
  "type": "typing"
}
```

No fields. The server broadcasts a `typing_indicator` to other clients. Typing indicators expire after 5 seconds without renewal. The server does not persist these.

#### `ping`

Application-level keepalive. Distinct from WebSocket protocol-level ping/pong frames (which axum handles automatically).

```json
{
  "type": "ping",
  "seq": 42
}
```

Fields:
- `seq` (integer, required): Sequence number. The server echoes it back in the `pong` response.

#### `read_cursor`

Informs the server which messages the client has seen. Optional. Useful for AI bots that want to track their read position across reconnections.

```json
{
  "type": "read_cursor",
  "last_ts": "2026-04-02T12:05:00Z"
}
```

Fields:
- `last_ts` (string, required): ISO 8601 timestamp of the most recent message the client has processed.

The server stores this per-connection (not persisted across reconnections in v1). Future versions may persist cursors by peer identity.

### Message Types: Server to Client

#### `welcome`

Sent in response to a successful `identify`.

```json
{
  "type": "welcome",
  "session_id": "a1b2c3d4-...",
  "identity": "claude:bot",
  "server_version": "0.1.0"
}
```

Fields:
- `session_id` (string): The active session's UUID.
- `identity` (string): The `from` identity the server will use for this client's messages. Format: `{name}:{client_type}`.
- `server_version` (string): bpmux version. Clients can use this for compatibility checks.

#### `history`

Sent immediately after `welcome`. Contains recent message history so the client has context.

```json
{
  "type": "history",
  "messages": [
    {"from": "ryan:human", "text": "hello", "ts": "2026-04-02T12:00:00Z"},
    {"from": "ryan:mcp-client", "text": "hi there", "ts": "2026-04-02T12:00:05Z"}
  ]
}
```

Fields:
- `messages` (array): The last N messages from the session log (default N=50, configurable). Each element matches the existing `Message` struct: `{from, text, ts}`.

If the client sent a `read_cursor` in a previous connection and the server recognizes the peer, the history starts from the cursor position. Otherwise, the last N messages.

#### `message`

A new chat message appeared in the session. Pushed to all connected clients whenever any participant (TUI, MCP, WebSocket, or future WebRTC) sends a message.

```json
{
  "type": "message",
  "data": {
    "from": "ryan:human",
    "text": "what do you think?",
    "ts": "2026-04-02T12:00:00Z"
  }
}
```

Fields:
- `data` (object): The `Message` struct. Same schema as the JSONL log and the MCP `read_messages` response.

This is the core real-time event. The `data` field uses the existing `Message` shape exactly -- no translation, no wrapper. The same struct serializes to the JSONL log, to MCP responses, and to WebSocket frames.

#### `peer_joined`

A new client connected and identified.

```json
{
  "type": "peer_joined",
  "identity": "claude:bot",
  "client_type": "bot"
}
```

#### `peer_left`

A client disconnected.

```json
{
  "type": "peer_left",
  "identity": "claude:bot"
}
```

#### `typing_indicator`

Another client is typing.

```json
{
  "type": "typing_indicator",
  "identity": "ryan:human"
}
```

Expires after 5 seconds without renewal. Clients should clear the indicator after the timeout.

#### `pong`

Response to a client `ping`.

```json
{
  "type": "pong",
  "seq": 42
}
```

#### `error`

Sent when the server rejects a client message.

```json
{
  "type": "error",
  "code": "not_identified",
  "message": "Must send identify before other messages"
}
```

Error codes:
- `not_identified` -- client sent a non-identify message before identifying
- `invalid_message` -- malformed JSON or unknown message type
- `text_too_long` -- message text exceeds 4096 bytes
- `rate_limited` -- client is sending messages too fast
- `name_taken` -- another connected client already uses this name

---

## 3. Interaction with the JSONL Log

The JSONL log (`messages-current.jsonl`) is the source of truth. The WebSocket is a real-time view over it.

### Writes

When the server receives a `send` message from a WebSocket client:
1. Construct a `Message` with the assigned `from` identity and current UTC timestamp.
2. Call `session::append_message()` to persist to the JSONL log.
3. Send the `Message` on the `broadcast::Sender<Message>` channel.
4. All subscribers (other WebSocket clients, TUI wake) receive the message.

This is identical to how the MCP `send_message` tool works today. The write path is shared.

### Reads

The `history` backfill on connect calls `session::read_messages()` -- the same function MCP's `read_messages` tool uses.

Real-time messages after connection come from the broadcast channel, not from re-reading the log. The log is only read on initial connect (for history) or on reconnect.

### Consistency

Because all writers go through `session::append_message()` and the broadcast channel, all clients see the same message stream regardless of source. A message sent from the TUI, from MCP, or from a WebSocket client all follow the same path: log append, then broadcast.

---

## 4. Broadcast Channel Changes

The existing broadcast channel carries `()` (a wake signal). To support WebSocket push, it must carry the actual `Message`.

### Current

```rust
let (wake_tx, wake_rx) = broadcast::channel::<()>(16);
```

The TUI event loop calls `wake_rx.try_recv().is_ok()` and re-reads the entire log.

### Proposed

```rust
let (msg_tx, _) = broadcast::channel::<session::Message>(64);
```

The TUI event loop receives the actual message and appends it to `app.messages` directly, without re-reading the log. WebSocket handlers receive the message and forward it as a JSON frame. The MCP server sends the message on the same channel after `append_message`.

This is a one-line type change in `src/lib.rs` and a small refactor of the TUI wake logic in `src/terminal/event.rs`. The `Message` struct already derives `Clone` (required by broadcast).

---

## 5. Server-Side Architecture

### Shared State

```rust
#[derive(Clone)]
struct AppState {
    session_dir: Arc<PathBuf>,
    msg_tx: broadcast::Sender<session::Message>,
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
}

struct PeerInfo {
    identity: String,
    client_type: String,
    connected_at: DateTime<Utc>,
}
```

The `AppState` is shared between the MCP server, the WebSocket handler, and (in future) the signaling server. It is passed via axum's `State` extractor.

### WebSocket Handler

```rust
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(socket: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Phase 1: Wait for identify
    // Read first message, expect type=identify
    // Validate name, register peer
    // Send welcome + history

    // Phase 2: Bidirectional relay
    let mut broadcast_rx = state.msg_tx.subscribe();

    // Task 1: broadcast -> WebSocket (push to client)
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            let envelope = json!({"type": "message", "data": msg});
            if ws_tx.send(WsMessage::Text(envelope.to_string())).await.is_err() {
                break;
            }
        }
    });

    // Task 2: WebSocket -> broadcast (receive from client)
    while let Some(Ok(frame)) = ws_rx.next().await {
        match frame {
            WsMessage::Text(text) => {
                // Parse, validate, construct Message, append_message, broadcast
            }
            WsMessage::Close(_) => break,
            _ => {} // Ignore binary, ping, pong (axum handles pong automatically)
        }
    }

    // Cleanup: unregister peer, broadcast peer_left
    send_task.abort();
}
```

This is approximately 80-120 lines of Rust. No new dependencies -- axum 0.8 WebSocket support, futures `StreamExt`/`SinkExt`, and tokio broadcast are all already in `Cargo.toml`.

### Rate Limiting

The server enforces a per-client rate limit of 10 messages per second. Excess messages receive an `error` with code `rate_limited`. This prevents runaway bots from flooding the session.

Implementation: a simple token bucket per connection (no external crate needed -- a counter plus a timestamp is sufficient).

---

## 6. Relationship to MCP

| Aspect | MCP (`/mcp`) | WebSocket (`/ws`) |
|---|---|---|
| Direction | Request/response | Bidirectional push |
| Use case | AI tool integration (Claude Code, Cursor) | Real-time participant (bot, Web UI) |
| Who connects | AI coding tools via MCP protocol | Dedicated bots, browsers, test harnesses |
| Message flow | Tool calls: `send_message`, `read_messages` | Push: server sends `message` events; client sends `send` |
| Latency | Polling interval (seconds) | Real-time (milliseconds) |
| Auth | MCP session token (future) | WebSocket query param or header (future) |
| Shared infra | Same JSONL log, same broadcast channel, same axum server |

Both coexist on the same port. Both write through `session::append_message()`. Both broadcast through the same `broadcast::Sender<Message>`. A message sent via MCP appears instantly on all WebSocket clients, and vice versa.

MCP remains the right choice for AI tools that have their own UX and workflow (Claude Code decides when to call `read_messages`). WebSocket is the right choice for dedicated participants that want to follow the conversation in real time.

---

## 7. Preparing for WebRTC (Story 5)

The WebSocket protocol between bpmux and local clients is distinct from WebRTC between remote peers, but they share the same message format.

### Same Message Shape

The `Message` struct (`{from, text, ts}`) is the canonical message shape everywhere:
- JSONL log on disk
- MCP `read_messages` response
- WebSocket `message` event `data` field
- WebRTC data channel payloads (Story 5)

When WebRTC is added, messages received from a remote peer arrive over the data channel, are written to the local JSONL log via `session::append_message()`, and broadcast on the local `broadcast::Sender<Message>`. Local WebSocket clients see remote messages through the same path as local messages. The source is transparent.

### Different Transports, Same Broadcast

```
Remote peer (WebRTC data channel)
    |
    v
session::append_message() + msg_tx.send()
    |
    v
broadcast::Sender<Message>
    |
    +---> TUI (updates display)
    +---> WebSocket client A (bot)
    +---> WebSocket client B (web UI)
    +---> MCP notification (future)
```

The broadcast channel is the internal fan-out point. WebSocket, WebRTC, MCP, and the TUI all connect to it. Adding WebRTC does not change the WebSocket protocol -- it adds another source of messages into the same broadcast.

### Signaling Is Separate

The WebRTC signaling protocol (SDP exchange, ICE candidates) uses a separate WebSocket connection to a signaling server, as specified in the signaling-patterns research. The `/ws` endpoint is for chat participation, not for WebRTC signaling. These are different concerns on different paths.

---

## 8. How an AI Bot Uses This

A minimal bot is a WebSocket client that connects, receives messages, decides when to respond, and sends responses back. The bot owns all AI-specific logic: provider calls, context management, trigger policy, system prompts. bpmux knows nothing about LLMs.

### Minimal Python Bot Example

```python
import asyncio
import json
import websockets
import anthropic

async def bot():
    uri = "ws://127.0.0.1:4117/ws"
    async with websockets.connect(uri) as ws:
        # Identify
        await ws.send(json.dumps({
            "type": "identify",
            "name": "claude",
            "client_type": "bot"
        }))

        # Receive welcome + history
        welcome = json.loads(await ws.recv())
        history = json.loads(await ws.recv())

        context = [
            {"role": "user", "content": m["text"]}
            if not m["from"].endswith(":bot")
            else {"role": "assistant", "content": m["text"]}
            for m in history["messages"]
        ]

        client = anthropic.Anthropic()

        # Message loop
        async for raw in ws:
            msg = json.loads(raw)
            if msg["type"] != "message":
                continue

            data = msg["data"]

            # Skip own messages (prevent loops)
            if data["from"] == "claude:bot":
                continue

            # Only respond when mentioned
            if "@claude" not in data["text"].lower():
                continue

            # Build context, call LLM
            context.append({"role": "user", "content": f'{data["from"]}: {data["text"]}'})
            response = client.messages.create(
                model="claude-sonnet-4-20250514",
                max_tokens=1024,
                system="You are a participant in a pair programming session.",
                messages=context[-20:]  # sliding window
            )

            reply = response.content[0].text
            context.append({"role": "assistant", "content": reply})

            # Send response
            await ws.send(json.dumps({
                "type": "send",
                "text": reply
            }))

asyncio.run(bot())
```

### Key Bot Patterns

**Loop prevention.** The bot must skip its own messages. The `from` field makes this trivial: `if data["from"] == "claude:bot": continue`.

**Trigger policy.** The bot decides when to speak. Common policies:
- Respond when mentioned by name (`@claude`)
- Respond to all human messages (aggressive)
- Respond only when asked a question (requires intent detection)
- Never respond unsolicited (passive observer that logs reasoning)

**Context windowing.** The bot maintains a sliding window of recent messages for LLM context. The history backfill provides initial context. Subsequent messages extend the window. The bot trims old messages to stay within token limits. This is the bot's concern, not bpmux's.

**Graceful reconnection.** If the WebSocket drops, the bot reconnects, re-identifies, and receives a fresh history backfill. The `read_cursor` message can be used to avoid reprocessing messages the bot already saw.

---

## 9. Patterns from Discord, Slack, and Matrix

### What bpmux adopts

**Tagged message types (all three).** Discord uses numeric opcodes, Slack and Matrix use string type fields. bpmux uses string types for readability. The `type` field on every frame is a universal pattern.

**Identify-then-communicate (Discord).** Discord requires an `Identify` payload before the gateway sends events. bpmux requires `identify` before accepting other messages. This establishes identity and prevents anonymous participation.

**History backfill on connect (Matrix).** Matrix's `/sync` endpoint returns recent state on initial sync. bpmux sends a `history` message after welcome. This gives late-joining clients context without polling.

**Presence events (Discord, Slack).** Discord sends `PRESENCE_UPDATE`, Slack sends `presence_change`. bpmux sends `peer_joined` and `peer_left`. These let clients show who is in the session.

**Typing indicators (all three).** All three platforms send typing events that expire after a timeout. bpmux's `typing` / `typing_indicator` follows the same pattern with a 5-second expiry.

**Server-assigned identity (Slack).** In Slack's RTM API, the server tells the client its user ID in the `hello` event. bpmux's `welcome` message tells the client its assigned `identity`. Clients cannot forge the `from` field.

### What bpmux does not adopt

**Numeric opcodes (Discord).** Discord uses opcode integers (0=Dispatch, 1=Heartbeat, etc.) for compactness. bpmux uses string types because human readability matters more than byte savings at this scale. A bpmux session has single-digit participants, not millions.

**Sequence numbers for resumption (Discord).** Discord's gateway assigns a sequence number to every event, and clients use it to resume after disconnection without missing events. bpmux uses timestamp-based cursor (`read_cursor`) instead, since the JSONL log is the source of truth and messages are already ordered by timestamp. If a client reconnects, it gets a history backfill. No sequence tracking needed on the server.

**Intents / event filtering (Discord).** Discord lets bots declare which event types they want via Gateway Intents. bpmux sends all events to all clients because the event volume is trivially small (a chat session, not a million-user platform). Filtering can be added later if needed.

**Channel multiplexing (Slack, Matrix).** Slack and Matrix support multiple channels/rooms per connection. bpmux has one session per server. No channel routing needed.

**HTTP long-polling fallback (Matrix).** Matrix supports long-polling for clients that cannot use WebSocket. bpmux does not need this -- the MCP endpoint already serves clients that prefer request/response.

---

## 10. Error Handling

### Malformed Messages

If the server cannot parse a JSON frame, it sends an `error` with code `invalid_message` and keeps the connection open. One bad frame does not kill the session.

### Connection Drops

WebSocket connections drop without notice (network failure, process crash). The server detects this when a send or receive fails. It unregisters the peer and broadcasts `peer_left`. No explicit close handshake is required.

### Broadcast Lag

If a WebSocket client cannot keep up with the broadcast rate (slow consumer), the `broadcast::Receiver` returns `RecvError::Lagged(n)`. The server logs this and continues -- the client misses `n` messages. For a chat session with human-speed typing, this is unlikely. For future high-throughput scenarios, a per-client bounded buffer with backpressure could replace the broadcast channel.

### Server Shutdown

On graceful shutdown (Ctrl-C, `/quit`), the server sends a WebSocket close frame to all connected clients before exiting. Clients should handle the close frame and reconnect or exit as appropriate.

---

## 11. Security Considerations

### v1 (localhost only)

- Server binds to `127.0.0.1`. Only local processes can connect.
- No authentication. Any local process can join any session.
- No encryption (plaintext WebSocket, not WSS). Acceptable on localhost.
- The `from` field is server-assigned. Clients cannot impersonate other participants.
- Rate limiting prevents a single client from flooding the session.
- Message size limit (4096 bytes) prevents memory exhaustion.

### Future (Story 6)

- Session token authentication on WebSocket upgrade.
- WSS (TLS) for non-localhost connections.
- Per-session access control: only clients with the session token can connect.
- Trust levels on messages based on `client_type` (matching Story 8's trust model).
- Messages from `bot` clients could be tagged as untrusted when relayed to MCP clients.

---

## 12. Implementation Scope

### Changes to existing code

1. **`src/lib.rs`**: Change `broadcast::channel::<()>(16)` to `broadcast::channel::<session::Message>(64)`. Update the MCP server to send the actual `Message` on broadcast instead of `()`.

2. **`src/terminal/event.rs`**: Change `wake_rx.try_recv().is_ok()` to receive `Message` and append directly to `app.messages` instead of re-reading the entire log.

3. **`src/mcp/server.rs`**: Update `handle_send_message` to send the `Message` on the broadcast channel (not just `()`). Update `AppState` to include shared state needed by both MCP and WebSocket handlers.

### New code

4. **`src/ws/mod.rs`** (or `src/peer/mod.rs`): WebSocket handler. The `ws_handler` function, `handle_ws` connection loop, identify/welcome/history sequence, bidirectional relay, peer tracking. Estimated 100-150 lines.

5. **Route registration**: One line in the axum router setup.

### No new dependencies

axum 0.8 includes `axum::extract::ws`. futures provides `StreamExt`/`SinkExt`. tokio provides `broadcast`. serde_json handles serialization. All already in `Cargo.toml`.

---

## 13. Message Type Summary

| Type | Direction | Purpose |
|---|---|---|
| `identify` | Client -> Server | Establish identity on connect |
| `send` | Client -> Server | Send a chat message |
| `typing` | Client -> Server | Signal that client is composing |
| `ping` | Client -> Server | Application-level keepalive |
| `read_cursor` | Client -> Server | Report read position |
| `welcome` | Server -> Client | Confirm identity, provide session info |
| `history` | Server -> Client | Backfill recent messages on connect |
| `message` | Server -> Client | Push new chat message |
| `peer_joined` | Server -> Client | New participant connected |
| `peer_left` | Server -> Client | Participant disconnected |
| `typing_indicator` | Server -> Client | Another participant is typing |
| `pong` | Server -> Client | Response to ping |
| `error` | Server -> Client | Reject invalid client message |

---

## 14. Wire Examples

### Full Connection Sequence

```
Client -> Server:  {"type":"identify","name":"claude","client_type":"bot"}
Server -> Client:  {"type":"welcome","session_id":"a1b2c3d4-...","identity":"claude:bot","server_version":"0.1.0"}
Server -> Client:  {"type":"history","messages":[{"from":"ryan:human","text":"hello","ts":"2026-04-02T12:00:00Z"}]}
Server -> Client:  {"type":"message","data":{"from":"ryan:human","text":"@claude what do you think?","ts":"2026-04-02T12:01:00Z"}}
Client -> Server:  {"type":"send","text":"I think the approach looks solid."}
Server -> Client:  {"type":"message","data":{"from":"claude:bot","text":"I think the approach looks solid.","ts":"2026-04-02T12:01:05Z"}}
```

### Peer Join/Leave

```
Server -> All:     {"type":"peer_joined","identity":"alice:web-ui","client_type":"web-ui"}
Server -> All:     {"type":"peer_left","identity":"alice:web-ui"}
```

### Error Case

```
Client -> Server:  {"type":"send","text":"...4097 bytes..."}
Server -> Client:  {"type":"error","code":"text_too_long","message":"Message text exceeds 4096 byte limit"}
```

---

## 15. Open Questions

1. **Should the TUI also connect via WebSocket?** Currently the TUI reads the broadcast channel directly (in-process). It could instead connect to its own WebSocket endpoint, making it a true client like any other. This adds latency and complexity for no user-visible benefit in v1, but it would validate the protocol and simplify the architecture (one path for all clients). Decision: defer to implementation time.

2. **Should system messages (join/leave) be written to the JSONL log?** Currently the log only contains chat messages. Adding system events would give reconnecting clients a complete audit trail but changes the log schema. Decision: yes, with a `"type"` field added to the log entries to distinguish chat messages from system events.

3. **Should the history backfill be configurable?** Default 50 messages. Should clients be able to request more or fewer? A `history_count` field in the `identify` message could support this. Decision: yes, add optional `history_count` to `identify`, default 50, max 500.

4. **Should the server echo `send` back to the sender?** The current design echoes: the sender receives its own message as a `message` event after the server processes it. This confirms delivery and provides the server-assigned timestamp. Discord and Slack both echo. The alternative (no echo, client optimistically displays) saves bandwidth but loses confirmation. Decision: echo, matching Discord/Slack behavior.

---

## Sources

- Discord Gateway API: https://docs.discord.com/developers/events/gateway
- Slack Socket Mode: https://docs.slack.dev/apis/events-api/using-socket-mode/
- Slack Legacy RTM: https://api.slack.com/legacy/rtm
- Matrix Client-Server API: https://spec.matrix.org/latest/client-server-api/
- axum WebSocket: https://docs.rs/axum/latest/axum/extract/ws/index.html
- Internal: `2026-04-02-external-peer-model.md`, `2026-04-02-design-reconciliation.md`, `2026-04-02-signaling-patterns.md`, `2026-04-02-story-candidates.md`
