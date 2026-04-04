# MCP Spec: Server-Initiated Communication Capabilities

Research into what the MCP specification actually defines for server-to-client push, what rmcp (v1.3.0) supports, and what Claude Code implements as a client.

## 1. Server-Initiated Messages in the MCP Spec

The MCP spec (2025-03-26 revision, with 2025-11-25 additions) defines several mechanisms for a server to communicate with a client without the client polling.

### Notifications (server to client)

Servers can send JSON-RPC notifications (fire-and-forget, no response expected) for:

- `notifications/resources/list_changed` -- the list of available resources changed
- `notifications/resources/updated` -- a specific subscribed resource changed (includes the URI)
- `notifications/tools/list_changed` -- the list of available tools changed
- `notifications/prompts/list_changed` -- the list of available prompts changed
- `notifications/message` -- a structured log message (with level, logger name, arbitrary JSON data)
- `notifications/progress` -- progress update for a long-running operation
- `notifications/cancelled` -- cancellation of a request

These are all predefined types. A server cannot send arbitrary custom notification types through the standard spec. However, rmcp defines a `CustomNotification` type for non-standard peer-to-peer messages.

### Requests (server to client)

The spec also defines requests that a server can send to the client (these expect a response):

- `sampling/createMessage` -- ask the client to perform an LLM completion
- `elicitation/create` -- ask the client to gather user input (form or URL mode)
- `roots/list` -- ask the client for its filesystem roots

These are genuine server-initiated requests, not notifications. They go the "wrong way" -- the server asks, the client answers.

### Summary of spec mechanisms

| Mechanism | Direction | Response? | Purpose |
|---|---|---|---|
| Notifications | Server -> Client | No | Resource changes, tool list changes, logging, progress |
| Sampling | Server -> Client | Yes (LLM output) | Server requests LLM completion from client |
| Elicitation | Server -> Client | Yes (user input) | Server requests structured user input |
| Resource subscriptions | Bidirectional | Mixed | Client subscribes, server notifies on change |

## 2. MCP Notifications in Detail

### Server-sent notifications

All defined server notification types:

1. **`notifications/resources/list_changed`** -- No params. Server must declare `resources.listChanged` capability.
2. **`notifications/resources/updated`** -- Params: `{ uri: string }`. Server must declare `resources.subscribe` capability. Client must have subscribed via `resources/subscribe`.
3. **`notifications/tools/list_changed`** -- No params. Server must declare `tools.listChanged` capability.
4. **`notifications/prompts/list_changed`** -- No params. Server must declare `prompts.listChanged` capability.
5. **`notifications/message`** -- Params: `{ level, logger, data }`. Server must declare `logging` capability. This is the most flexible notification type -- `data` is arbitrary JSON.
6. **`notifications/progress`** -- Params: `{ progressToken, progress, total }`. For in-flight operations.
7. **`notifications/cancelled`** -- Params: `{ requestId, reason }`. Cancellation signal.

### Client-sent notifications

For completeness, clients can send:

1. **`notifications/initialized`** -- Signals initialization complete
2. **`notifications/roots/list_changed`** -- Root list changed (client must declare `roots.listChanged`)
3. **`notifications/cancelled`** -- Client cancels a request
4. **`notifications/progress`** -- Client reports progress

### Can a server send arbitrary notifications?

No, not in the standard spec. The notification types are predefined. The closest thing to arbitrary server-to-client data push is `notifications/message` (logging), which accepts arbitrary JSON in the `data` field. But clients typically treat these as log output, not as actionable events.

## 3. Resource Subscriptions

The spec defines a subscription mechanism for resources:

1. Server declares `resources: { subscribe: true }` capability
2. Client sends `resources/subscribe` with a URI
3. When the resource changes, server sends `notifications/resources/updated` with that URI
4. Client then calls `resources/read` to fetch the updated content

This is a genuine push notification -- the server decides when to notify. The client does not poll.

### Could bpmux's message log be a subscribed resource?

Yes, in theory. bpmux could expose the message log as a resource (e.g., `bpmux://session/messages`) and send `notifications/resources/updated` when new messages arrive. The client would then re-read the resource to get the latest content.

**However**: the client only knows the resource changed, not what changed. It must re-read the entire resource. For a growing message log, this means re-reading the entire log on every new message. There is no delta/diff mechanism in the spec.

**Practical limitation**: Claude Code's implementation of resource subscriptions is unclear. The spec defines it, but whether Claude Code actually subscribes to resources and reacts to update notifications is a separate question (see section 8).

## 4. MCP Sampling

Sampling (`sampling/createMessage`) lets the server request the client to perform an LLM generation.

### How it works

1. Server sends `sampling/createMessage` with messages, model preferences, system prompt, and max tokens
2. Client presents the request to the user for approval (human-in-the-loop)
3. Client forwards the approved prompt to an LLM
4. Client returns the LLM's response to the server

The client declares `sampling: {}` capability during initialization. The server can include model hints (e.g., prefer Claude Sonnet) and priority weights (cost, speed, intelligence).

### Is sampling implemented in Claude Code?

**No.** As of the latest available information (early 2026), Claude Code does not support sampling. GitHub issue #1785 (opened June 2025) is an open feature request. The issue has 103+ upvotes and Anthropic staff have acknowledged it but it remains unimplemented.

VS Code's Copilot client implemented sampling support earlier (mid-2025).

### Could bpmux use sampling?

If sampling were implemented, bpmux could:
- Detect a new message in the chat
- Send `sampling/createMessage` with the message context
- The client (Claude Code) would generate a response
- bpmux would receive the response and post it to the chat

This would be the most natural "push" mechanism for bpmux -- it directly triggers the AI to think about new information. But it is not available in Claude Code today.

## 5. MCP Elicitation

Elicitation (`elicitation/create`) lets the server request structured input from the user.

### How it works

Two modes:

1. **Form mode**: Server sends a JSON schema, client renders a form, user fills it in
2. **URL mode**: Server sends a URL, client opens it in a browser (for sensitive data like API keys)

The client declares `elicitation: { form: {}, url: {} }` capability.

### Is elicitation implemented in Claude Code?

**Yes.** Claude Code v2.1.76 (March 2026) added elicitation support. MCP servers can now pause execution and request structured input from the user via an interactive dialog.

### Could bpmux use elicitation?

Elicitation is designed for gathering input during a tool call, not for pushing unsolicited events. It would not solve the "new message arrived, please respond" problem. It could be useful for configuration (e.g., asking the user which session to join), but not for real-time message delivery.

### Elicitation spec version note

Form mode was introduced in MCP spec 2025-06-18. URL mode was added in 2025-11-25. Both are in the current spec.

## 6. Streamable HTTP Transport

bpmux uses rmcp's `StreamableHttpService`. The Streamable HTTP transport (introduced in spec revision 2025-03-26, replacing the deprecated HTTP+SSE transport) supports server-to-client push.

### How server push works over Streamable HTTP

Two mechanisms:

**Mechanism 1: SSE responses to POST requests.** When a client sends a POST (e.g., a tool call), the server can respond with `Content-Type: text/event-stream` and stream multiple messages back -- including server-initiated requests and notifications -- before sending the final response.

**Mechanism 2: GET-based SSE stream.** The client can issue an HTTP GET to the MCP endpoint to open a persistent SSE stream. The server can then push JSON-RPC requests and notifications on this stream at any time, unrelated to any client request.

From the spec:
- "The client MAY issue an HTTP GET to the MCP endpoint. This can be used to open an SSE stream, allowing the server to communicate to the client, without the client first sending data via HTTP POST."
- "The server MAY send JSON-RPC requests and notifications on the stream."

### Is this truly bidirectional?

Yes. The GET-based SSE stream is a genuine server-push channel. The server can send notifications and requests at any time. The client can have multiple SSE streams open simultaneously.

### Session management

The transport supports session IDs (`Mcp-Session-Id` header) for stateful connections, and resumability via SSE event IDs and `Last-Event-ID` headers.

### Key point for bpmux

The Streamable HTTP transport provides the wire-level capability for server push. Whether the client (Claude Code) actually opens a GET-based SSE stream and processes unsolicited server notifications depends on the client implementation.

## 7. What rmcp (v1.3.0) Supports

rmcp is the Rust MCP SDK used by bpmux (`rmcp = { version = "1.3.0", features = ["server", "client", "transport-async-rw", "transport-streamable-http-server", "transport-streamable-http-client-reqwest"] }`).

### Server-initiated communication in rmcp

The `Peer` struct (accessible from a server handler) exposes:

**Notification methods (server-only):**
- `notify_resource_list_changed()`
- `notify_resource_updated()`
- `notify_tool_list_changed()`
- `notify_prompt_list_changed()`
- `notify_logging_message()`
- `notify_url_elicitation_completed()`

**Request methods (server to client):**
- `create_message()` -- sampling request
- `create_elicitation()` -- elicitation request
- `list_roots()` -- roots request

**Generic methods:**
- `send_notification()` -- send any notification
- `send_request()` -- send any request and await response

**Types defined:**
- `ResourceUpdatedNotification`
- `ResourceListChangedNotification`
- `ToolListChangedNotification`
- `PromptListChangedNotification`
- `LoggingMessageNotification`
- `ProgressNotification`
- `CancelledNotification`
- `CustomNotification` -- for non-standard notifications
- `CreateMessageRequest` / `CreateMessageResult` -- sampling
- `CreateElicitationRequest` / `CreateElicitationResult` -- elicitation
- `ElicitationSchema` / `ElicitationCapability`

### Summary

rmcp fully supports all MCP server-initiated communication mechanisms: notifications, sampling, elicitation, and resource subscriptions. It also provides `CustomNotification` for non-standard use cases and generic `send_notification()`/`send_request()` for anything else.

## 8. What Claude Code Actually Implements

This is the critical section. The spec defines mechanisms; rmcp implements them. But what does Claude Code, as the MCP client, actually support?

### Confirmed implemented

- **Tools**: Full support. Claude Code discovers, lists, and calls tools.
- **Resources**: Claude Code can list and read resources.
- **Prompts**: Claude Code can list and use prompts.
- **Elicitation (form mode)**: Added in v2.1.76 (March 2026). MCP servers can request structured input.
- **Logging notifications**: Claude Code can receive `notifications/message` (log messages). These appear in debug logs.
- **Tool/resource/prompt list change notifications**: Claude Code likely handles these to refresh its view of available server capabilities.

### Confirmed NOT implemented

- **Sampling**: Not implemented. Open feature request (issue #1785, June 2025). 103+ upvotes.
- **Elicitation URL mode**: Unclear. The spec added URL mode in 2025-11-25; Claude Code added elicitation in March 2026. May only support form mode.

### Unclear / likely not implemented

- **Resource subscriptions**: The spec defines `resources/subscribe` and `notifications/resources/updated`. Whether Claude Code sends subscription requests and reacts to update notifications is not documented. Given that Claude Code's MCP usage is primarily tool-oriented, resource subscriptions are likely not implemented or not actively used.
- **GET-based SSE stream for server push**: For stdio transport (which Claude Code uses for local servers), this is not relevant -- stdio is inherently bidirectional. For HTTP transport, it is unclear whether Claude Code opens a GET SSE stream to receive unsolicited server messages.

### Claude Code Channels (research preview)

Claude Code v2.1.80 introduced "channels" -- a mechanism for MCP servers to push events into a running session. This is the most relevant feature for bpmux.

**How channels work:**
1. An MCP server declares `experimental: { 'claude/channel': {} }` capability
2. The server sends `notifications/claude/channel` notifications with content and metadata
3. Claude Code receives these and injects them into the conversation as `<channel source="...">` tags
4. Claude reads the injected content and can act on it
5. Two-way channels also expose tools for Claude to reply back

**Key details:**
- Channels are a Claude Code extension, not part of the MCP spec. They use the spec's notification mechanism but with a custom notification type (`notifications/claude/channel`).
- Channel servers connect over stdio (spawned as subprocesses).
- Events only arrive while the session is open.
- Channels are in research preview (v2.1.80+, requires claude.ai login).
- The feature includes permission relay for remote tool approval.
- Pre-built channels exist for Telegram, Discord, iMessage, and a fakechat demo.
- Custom channels require `--dangerously-load-development-channels` during the preview.

**This is exactly what bpmux needs.** A channel is an MCP server that pushes events into a Claude Code session -- precisely the server-to-client push mechanism we need.

## Analysis for bpmux

### What mechanisms could bpmux use?

| Mechanism | Available? | Practical? | Notes |
|---|---|---|---|
| Resource subscriptions | Spec: yes. Claude Code: unclear | Maybe | Re-reads entire resource on each update. No delta. |
| Sampling | Spec: yes. rmcp: yes. Claude Code: no | No | Not implemented in Claude Code. |
| Elicitation | Spec: yes. Claude Code: yes (form) | No | Wrong tool -- designed for gathering input, not pushing events. |
| Logging notifications | Spec: yes. Claude Code: yes | Hacky | Could abuse log messages to carry data. Client treats as logs. |
| Channels | Claude Code extension | Yes | Purpose-built for pushing events into sessions. Research preview. |
| Tool polling | Always works | Yes but wasteful | Client calls a tool periodically. Reliable but burns tokens. |

### The channels path

Channels are the obvious answer. They are:
- Purpose-built for the exact use case (pushing external events into a Claude Code session)
- Already implemented and working (research preview)
- Based on standard MCP notifications (just a custom notification type)
- Documented with examples including chat bridges (two-way) and webhook receivers (one-way)

The constraint is that channels are in research preview, require `--channels` flag, and custom channels need `--dangerously-load-development-channels`.

### The resource subscription path

If channels are too experimental or too tightly coupled to Claude Code, resource subscriptions are the spec-standard alternative. bpmux could:
1. Expose the message log as a resource
2. Send `notifications/resources/updated` when new messages arrive
3. Hope the client re-reads the resource

This is spec-compliant but depends on the client implementing subscriptions. It also re-reads the entire resource each time.

### The tool polling path

The most reliable but least efficient approach. bpmux exposes a `check_messages` tool. Claude Code calls it periodically. Always works, no special features needed, but wastes tokens on empty polls.

### The hybrid path

bpmux could implement multiple mechanisms:
1. Channels for Claude Code (optimal experience)
2. Resource subscriptions for spec-compliant clients
3. Tool-based message reading as a fallback

The MCP server can declare capabilities at initialization and adapt based on what the client supports.

## Key Findings

1. **The MCP spec defines server-to-client push** via notifications, sampling, and elicitation. The Streamable HTTP transport supports genuine server push via SSE.

2. **rmcp (v1.3.0) fully supports all of these** including notifications, sampling, elicitation, and custom notifications.

3. **Claude Code's implementation is selective.** It supports tools, resources, prompts, elicitation (form), and logging. It does NOT support sampling. Resource subscription support is unclear.

4. **Claude Code Channels are the answer.** Introduced in v2.1.80 (research preview), channels are purpose-built for pushing events into a running Claude Code session. They use a custom notification type (`notifications/claude/channel`) over stdio transport.

5. **Sampling would be ideal but is unavailable.** If Claude Code implemented sampling, bpmux could push context and trigger the AI to respond. This is an open feature request with strong community support.

6. **The spec has no arbitrary notification type.** Servers cannot send custom notification types in the standard spec. Claude Code channels work around this with an experimental capability.

7. **Transport supports push; client support varies.** The wire protocol (Streamable HTTP with SSE, or stdio) fully supports bidirectional communication. The limiting factor is what the client implements.
