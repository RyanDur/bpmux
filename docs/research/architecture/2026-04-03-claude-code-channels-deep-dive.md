# Claude Code Channels: Deep Dive

## What This Is

Research into Claude Code Channels -- the MCP feature that lets servers push messages directly into a running Claude Code session. Channels were identified as potentially the key mechanism for bpmux to stay communication-only while getting Claude Code to participate as a first-class peer.

## 1. What Are Claude Code Channels?

Channels are an MCP extension that inverts the traditional passive server model. Instead of Claude Code polling or requesting data, the MCP server **pushes events into the session**. Claude sees them immediately and can act on them.

**Shipped:** v2.1.80, March 19, 2026. Research preview status. Requires claude.ai login (not API key auth). Team/Enterprise orgs must explicitly enable via `channelsEnabled` managed setting.

**How it works:**
1. An MCP server declares `claude/channel` in its experimental capabilities
2. Claude Code registers a notification listener for that server
3. The server emits `notifications/claude/channel` events whenever something happens
4. The event arrives in Claude's context as a `<channel source="server-name" ...>` XML tag
5. Claude reads it and acts -- calling tools, writing files, whatever the event warrants
6. For two-way channels, the server exposes a reply tool (standard MCP tool) that Claude calls to send messages back

**What Claude sees:**
```
<channel source="bpmux" from="ryan" sent_at="2026-04-03T10:30:00Z">
hey, can you look at the failing test in session/mod.rs?
</channel>
```

The `source` attribute comes from the server name. Every key in the `meta` object becomes an attribute on the tag.

## 2. The MCP Capability Declaration

The server declares channel support in its capabilities during the MCP initialization handshake:

```typescript
capabilities: {
  experimental: {
    'claude/channel': {},           // Required. Registers the notification listener.
    'claude/channel/permission': {} // Optional. Enables permission relay (v2.1.81+).
  },
  tools: {},  // Include if two-way (server exposes reply tool)
}
```

The key is `capabilities.experimental['claude/channel']`. The value is always an empty object `{}`. Its mere presence is what tells Claude Code to register a notification listener.

There is also an `instructions` field on the Server constructor that gets injected into Claude's system prompt. This is where you tell Claude how to interpret the events and how to reply.

## 3. Notification Format (Push Protocol)

When pushing a message, the server calls:

```typescript
await mcp.notification({
  method: 'notifications/claude/channel',
  params: {
    content: 'the message text',           // string, becomes body of <channel> tag
    meta: {                                 // optional, each key becomes a tag attribute
      from: 'ryan',
      chat_id: '42',
      severity: 'high'
    }
  }
})
```

**Constraints on `meta`:**
- Keys must be identifiers: letters, digits, underscores only
- Keys containing hyphens or other characters are silently dropped
- Values are strings

**What arrives in Claude's context:**
```
<channel source="your-server" from="ryan" chat_id="42" severity="high">
the message text
</channel>
```

## 4. How Claude Code Responds to Channel Messages

When a channel message arrives:
- Claude sees it immediately in its context (no user action needed)
- Claude processes it using the full local environment (files, tools, git, everything)
- The user sees the inbound message in their terminal
- If the channel is two-way, Claude calls the server's reply tool to send a response
- The user does NOT see the reply text in the terminal -- they see the tool call and "sent" confirmation. The actual reply appears on the other platform.

**Automatic behavior:** Claude does respond automatically to channel messages. The claude-peers-mcp instructions explicitly say "RESPOND IMMEDIATELY. Do not wait until your current task is finished."

**Server control:** The server controls behavior through the `instructions` string injected into the system prompt. You can tell Claude to just read and act (one-way), or to reply via a specific tool (two-way). The instructions shape Claude's behavior.

**Permission prompts:** If Claude tries to run a tool that needs approval (Bash, Write, etc.), the session pauses until the user responds at the terminal. Channels with `claude/channel/permission` capability can relay these prompts remotely.

## 5. The Reply Tool (Two-Way Channels)

For two-way communication, the server exposes a standard MCP tool. Nothing channel-specific about it:

```typescript
tools: [{
  name: 'reply',
  description: 'Send a message back over this channel',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'The conversation to reply in' },
      text: { type: 'string', description: 'The message to send' },
    },
    required: ['chat_id', 'text'],
  },
}]
```

The `instructions` tell Claude to pass the `chat_id` from the inbound `<channel>` tag back through the reply tool, so the server knows which conversation to respond in.

## 6. claude-peers-mcp: Proof of Concept

**Repository:** https://github.com/louislva/claude-peers-mcp
**Author:** louislva
**Language:** TypeScript (Bun runtime)
**Architecture:** Broker + per-instance MCP servers

### How It Works

```
                Broker Daemon
              localhost:7899 + SQLite
              /       |       \
         MCP Server  MCP Server  MCP Server
         (stdio)     (stdio)     (stdio)
           |           |           |
         Claude A    Claude B    Claude C
```

Each Claude Code session spawns an MCP server via stdio. The server registers with a shared broker daemon (auto-started, runs on localhost:7899 with SQLite). The server polls the broker every 1 second for new messages. When a message arrives, it pushes it into the Claude session via `notifications/claude/channel`.

### Key Files
- `server.ts` -- MCP server (spawned per Claude instance, ~350 lines)
- `broker.ts` -- HTTP broker daemon with SQLite
- `cli.ts` -- CLI for status/send/kill
- `shared/types.ts` -- Shared type definitions

### Tools Exposed
- `list_peers` -- Discover other Claude instances (scope: machine/directory/repo)
- `send_message` -- Send a message to a peer by ID
- `set_summary` -- Describe what you're working on
- `check_messages` -- Manual fallback for message retrieval

### Channel Push Implementation

The polling loop fetches messages from the broker, enriches them with sender metadata, then pushes each via channel notification:

```typescript
await mcp.notification({
  method: "notifications/claude/channel",
  params: {
    content: msg.text,
    meta: {
      from_id: msg.from_id,
      from_summary: fromSummary,
      from_cwd: fromCwd,
      sent_at: msg.sent_at,
    },
  },
});
```

### Launch Command
```bash
claude --dangerously-load-development-channels server:claude-peers
```

Requires: Bun, Claude Code v2.1.80+, claude.ai login.

## 7. Transport Constraint: Channels Require stdio

**This is the critical architectural finding.**

Channels require the **stdio transport**. Claude Code spawns the channel server as a subprocess and communicates over stdin/stdout. The official docs are explicit: "Your server needs to connect over stdio transport (Claude Code spawns your server as a subprocess)."

**bpmux currently uses Streamable HTTP transport** (axum + StreamableHttpService). This is fundamentally incompatible with channels. An HTTP server sitting on a port cannot push channel notifications into Claude Code -- Claude Code doesn't connect to external HTTP servers for channel events.

### What This Means for bpmux

bpmux cannot simply "declare channels" on its existing HTTP server. The architecture would need to change in one of these ways:

**Option A: Separate stdio sidecar**
bpmux runs its TUI + HTTP server as today. A thin stdio MCP server (spawned by Claude Code) connects to bpmux over localhost HTTP or WebSocket and bridges messages as channel notifications. This is essentially what claude-peers-mcp does -- the stdio server is a thin bridge to the broker.

**Option B: bpmux itself becomes the stdio server**
Claude Code spawns bpmux as a subprocess. bpmux communicates with Claude Code over stdio (MCP) and with other peers over WebSocket/HTTP. The TUI would need to run on a separate connection (or bpmux wouldn't have a TUI at all -- it would be headless when serving as a channel).

**Option C: Plugin wrapper**
A small TypeScript/Bun plugin acts as the stdio bridge. Installed via Claude Code's plugin system. Connects to bpmux's existing WebSocket endpoint. Pushes messages as channel notifications. This is the lightest touch -- bpmux doesn't change at all, the plugin is a ~100 line bridge.

## 8. Implementation in Rust with rmcp

The rmcp crate (v1.3.0, which bpmux already uses) **does support** the necessary primitives:

### Declaring Channel Capability

`ServerCapabilities` has an `experimental` field of type `Option<ExperimentalCapabilities>`, where `ExperimentalCapabilities` is `BTreeMap<String, JsonObject>`:

```rust
use rmcp::model::{ServerCapabilities, ServerInfo};
use serde_json::json;
use std::collections::BTreeMap;

let mut experimental = BTreeMap::new();
experimental.insert(
    "claude/channel".to_string(),
    serde_json::Map::new(), // empty object
);

let capabilities = ServerCapabilities {
    experimental: Some(experimental),
    tools: Some(Default::default()), // if two-way
    ..Default::default()
};

ServerInfo::new(capabilities)
    .with_instructions("Messages arrive as <channel source=\"bpmux\" ...>. Reply with send_message.")
```

### Sending Channel Notifications

`ServerNotification` has a `CustomNotification` variant. `CustomNotification` has fields:
- `method: String`
- `params: Option<serde_json::Value>`

```rust
use rmcp::model::{CustomNotification, ServerNotification};

let notification = CustomNotification::new(
    "notifications/claude/channel",
    Some(json!({
        "content": "hey, the tests are failing",
        "meta": {
            "from": "ryan",
            "sent_at": "2026-04-03T10:30:00Z"
        }
    })),
);

// Send via the peer handle
peer.send_notification(ServerNotification::CustomNotification(notification)).await?;
```

### The stdio Transport Challenge

rmcp supports stdio transport via `transport-async-rw` feature (which bpmux already enables). But bpmux's current `ServerHandler` implementation would need to be adapted. The stdio server would need:

1. A way to receive messages from the bpmux session (subscribe to the broadcast channel or poll the JSONL log)
2. A way to push those messages as channel notifications
3. Access to the `Peer<RoleServer>` handle to call `send_notification`

The `RequestContext<RoleServer>` provides a `peer` field, but you only get it during tool calls. For proactive push, you'd need to capture the peer handle during initialization or use a different pattern.

### Capturing the Peer Handle

The `ServerHandler` trait's tool handlers receive `RequestContext` with a `peer` field. To push notifications outside of tool calls, you'd likely need to:

1. Store the peer handle when the first request comes in
2. Use it from a background task that watches for new messages
3. Push channel notifications when messages arrive

This is architecturally similar to what claude-peers-mcp does with its polling loop.

## 9. The Integration Path

If bpmux adopts channels (Option A or C from above), the sequence would be:

1. **bpmux starts** normally with TUI + HTTP/WebSocket server on localhost
2. **User starts Claude Code** with `--dangerously-load-development-channels server:bpmux-channel`
3. **Claude Code spawns the stdio bridge** (either a thin TypeScript plugin or a Rust binary)
4. **Bridge connects to bpmux** over WebSocket (existing endpoint)
5. **Bridge declares `claude/channel`** during MCP init
6. **Claude gets instructions** via the `instructions` field: what events mean, how to reply
7. **When a message arrives** in the bpmux session (from TUI, from another Claude, from WebSocket):
   - bpmux broadcasts it via WebSocket
   - Bridge receives it
   - Bridge pushes `notifications/claude/channel` with content + meta
   - Claude sees `<channel source="bpmux-channel" from="ryan" ...>message</channel>`
8. **Claude responds** by calling `send_message` tool on the bridge
9. **Bridge forwards** to bpmux via WebSocket or HTTP
10. **Message appears** in TUI and is persisted to JSONL log

### Permission Handling

When Claude wants to run a tool that needs approval:
- If user is at the terminal: they approve locally
- If the bridge declares `claude/channel/permission`: the prompt gets forwarded to the bpmux TUI, and the user can approve from there
- If `--dangerously-skip-permissions` is set: no approval needed (for fully autonomous operation)

## 10. Limitations

**Research preview caveats:**
- `--channels` flag syntax and protocol may change
- Custom channels require `--dangerously-load-development-channels` (not on allowlist)
- Only works with claude.ai login, not API keys or Console auth
- Team/Enterprise orgs must explicitly enable

**Technical limitations:**
- Events only arrive while the session is open (no queuing/persistence on the Claude Code side)
- Channel messages are text-only (string content + string meta attributes)
- No structured data in the `<channel>` tag body -- it's a string
- Meta keys must be alphanumeric + underscore only (no hyphens)
- Rate limits are not documented, but presumably bounded by MCP notification throughput
- Works in CLI; unclear about Desktop app support for channels specifically

**Architectural constraints:**
- stdio transport required (server must be spawned as subprocess)
- One session per channel server instance (not shared across sessions)
- The channel server runs on the same machine as Claude Code

## 11. Verification: Is This Real?

**Confirmed real and shipped.** Evidence:

1. **Official Anthropic docs** at code.claude.com/docs/en/channels and code.claude.com/docs/en/channels-reference with complete protocol specification
2. **Claude Code v2.1.80 changelog** (March 19, 2026) lists `--channels (research preview)` as a headline feature
3. **Official plugin repository** at github.com/anthropics/claude-plugins-official with Telegram, Discord, iMessage, and fakechat implementations
4. **Third-party implementations** like claude-peers-mcp demonstrating the protocol works
5. **Multiple independent blog posts and tutorials** confirming hands-on usage

The previous research agent's claim about v2.1.80 and March 2026 was accurate.

## 12. Implications for bpmux

Channels are the mechanism that makes bpmux's "communication-only" design viable for AI participation. Without channels, Claude Code can only participate by polling (calling `read_messages` periodically), which is clunky and wasteful. With channels, messages arrive instantly in Claude's context and Claude can respond immediately.

**The bridge pattern (Option C) is the lightest path:**
- bpmux doesn't change its architecture at all
- A thin plugin (~100 lines TypeScript, or a small Rust binary) bridges WebSocket to stdio
- Claude Code spawns the bridge, bridge connects to bpmux's existing WebSocket
- Messages flow both ways with low latency

**The cost:**
- An extra process per Claude Code session
- The bridge needs to be installed/configured
- During research preview, requires `--dangerously-load-development-channels`
- Only works with claude.ai login

**What bpmux already has that makes this work:**
- WebSocket endpoint (`/ws`) for real-time message streaming
- `send_message` tool on the MCP server
- Broadcast channel (`wake_tx`) for internal message distribution
- JSONL persistence for message history

The missing piece is literally just the stdio-to-WebSocket bridge that declares `claude/channel`.
