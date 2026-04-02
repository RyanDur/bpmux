# Story Candidates for bpmux v1

*Draft — for discussion, not final. Stories are thin vertical slices of business value. Titles describe the value, not the technical work. Order is suggested but negotiable.*

---

## Story 0: A developer can start building bpmux

**Value:** The team can begin implementation.
**Expected behaviors:**
- `cargo build` succeeds
- `cargo test` runs (even if no tests yet)
- `cargo clippy` and `cargo fmt` pass
- Pre-push hook gates quality
- CLAUDE.md provides codebase guidance to AI sessions
- BSL license is in place

**Notes:** Repo scaffolding — Cargo.toml, src/main.rs, .github/workflows, pre-push hook, CLAUDE.md, LICENSE. AGENT_WORKFLOW.md already exists.

---

## Story 1: A developer can start a solo session and see the chat

**Value:** A single developer has a persistent conversation space that survives across AI sessions.
**Expected behaviors:**
- Running `bpmux` opens a TUI with a chat pane, input pane, and status bar
- The human can type a message and see it appear in the chat
- Messages are persisted to `~/.bpmux/sessions/<id>/messages.jsonl`
- Restarting `bpmux` and reconnecting to the same session shows the previous messages
- Status bar shows "solo" mode and session info

**Notes:** This is the thinnest possible slice — no MCP, no WebRTC, no Web UI, no encryption. Just the TUI, the message log, and human input. Touches: `terminal/`, `session/`, `main.rs`, `cli/` (for args parsing).

---

## Story 2: An AI tool can send and receive messages through bpmux

**Value:** The AI participates in the conversation — the developer and their AI share a persistent chat.
**Expected behaviors:**
- An MCP server starts on `127.0.0.1:4117` when a session begins
- An MCP client (Claude Code) can connect and call `send_message`
- Messages from the AI appear in the TUI with the AI's sender identity (e.g., `ryan:claude-code`)
- The AI can call `read_messages` and receive the message history
- The read cursor tracks what the AI has seen — calling `read_messages` again returns only new messages

**Notes:** Adds the MCP server (rmcp) alongside the existing TUI and session log. This is the first time two participants (human + AI) are on the same channel. Touches: `mcp/`, `session/` (read cursor).

---

## Story 3: A developer can see the chat in their browser

**Value:** Desktop users (Claude Desktop) can see and participate in the conversation without a terminal.
**Expected behaviors:**
- A Web UI is served on `localhost:4118` when a session begins
- Opening the URL in a browser shows the same chat as the TUI — messages, sender identities, timestamps
- The human can type and send messages from the Web UI
- Messages sent from the Web UI appear in the TUI and vice versa
- New messages appear in real-time (WebSocket, no polling)

**Notes:** Adds the axum HTTP server with embedded HTML/CSS/JS and a WebSocket for live updates. The Web UI reads from the same session log as the TUI. Touches: `web/`, `session/` (WebSocket wake notifications).

---

## Story 4: An AI can start a session without the developer touching a terminal

**Value:** Claude Desktop users can create and join sessions entirely through conversation with their AI.
**Expected behaviors:**
- The MCP server exposes `create_session` and `join_session` tools
- A Claude Desktop user says "start a bpmux session" and the AI calls `create_session` → session starts, returns session code
- The AI calls `open_chat` → returns Web UI URL → renders in Desktop preview panel
- `session_status` returns current session state (solo, connected, clients)

**Notes:** Adds session management MCP tools. The node must be able to start via MCP call, not just CLI. Touches: `mcp/` (new tools), `session/` (session lifecycle management).

---

## Story 5: Two developers can connect and chat in real-time

**Value:** Two people (and their AIs) share a conversation over the internet.
**Expected behaviors:**
- Dev A runs `bpmux create --name ryan` — gets a session code
- Dev B runs `bpmux join --name alice <code>` — connects via WebRTC
- Messages sent by either dev (TUI, Web UI, or MCP) appear on both sides
- Sender identities show the peer name (`ryan:human`, `alice:claude-code`)
- Disconnection is visible in the status bar; reconnection is automatic

**Notes:** The big one — adds WebRTC (str0m), signaling (axum WebSocket server + tokio-tungstenite client), and P2P message relay. Touches: `transport/`, `signaling/`, and integration with the existing `session/` broadcast channel.

---

## Story 6: A developer's conversation is secure

**Value:** Messages can't be intercepted, and only authorized participants can join or read.
**Expected behaviors:**
- WebRTC data channel is encrypted in transit (DTLS — built into str0m)
- Session log is encrypted at rest (AES-256-GCM, passphrase-derived key)
- Session codes are single-use — once a peer joins, the code is invalidated
- MCP server requires a session token on `initialize` — unauthorized clients are rejected
- Control socket has user-only permissions (mode 600)

**Notes:** Security features layered onto existing stories. Some (DTLS) come free with WebRTC. Others (at-rest encryption, session tokens, socket permissions) are explicit implementations. Touches: `session/` (encryption), `mcp/` (auth), `signaling/` (single-use codes).

---

## Story 7: An AI respects the context budget

**Value:** Using bpmux doesn't drain the developer's token budget.
**Expected behaviors:**
- `read_messages` returns only unread messages by default (server-side cursor)
- New clients fall back to last 50 messages (configurable)
- Messages are capped at 4KB
- Token budget is configurable via `~/.bpmux/config.toml` or `--token-budget` flag
- First-time setup prompts the user to choose subscription vs API key profile

**Notes:** Token awareness — the bounded windows, read cursor, config system, and first-time setup flow. Touches: `session/` (cursor, config), `mcp/` (parameter handling).

---

## Story 8: A developer knows who to trust in the chat

**Value:** The developer can control how much their AI trusts messages from other participants.
**Expected behaviors:**
- Each message has a trust level based on sender type (local, local-ai, peer, peer-ai)
- Peer AI messages are wrapped with `[UNTRUSTED]` when delivered via `read_messages`
- `/trust` and `/untrust` commands adjust trust levels
- Injection pattern detection flags suspicious messages with a warning field
- Trust changes take effect on subsequent `read_messages` calls

**Notes:** LLM security — trust levels, sanitization, role impersonation blocking. Touches: `session/` (trust state), `mcp/` (message wrapping), `terminal/` (trust commands).

---

## Story 9: bpmux deploys reliably

**Value:** The team can release bpmux binaries that are tested and trustworthy.
**Expected behaviors:**
- Push to main triggers CI (fmt, clippy, test, audit, deny)
- E2e tests spawn real binaries and test full workflows
- Version tags trigger release builds for macOS, Linux, Windows
- Smoke test verifies the binary starts, serves, and relays a message
- Release binaries are attached to GitHub Releases

**Notes:** CI/CD pipeline — GitHub Actions workflows, smoke test, cross-platform builds. Modeled after the battleship project's pipeline.

---

## Sequencing Notes

The stories are ordered to build incrementally:

```
Story 0 (scaffolding)
  └→ Story 1 (TUI + log — the thinnest usable slice)
       └→ Story 2 (MCP — AI joins the chat)
       └→ Story 3 (Web UI — Desktop users join)
            └→ Story 4 (MCP session management — zero-terminal flow)
       └→ Story 5 (WebRTC — P2P pairing)
            └→ Story 6 (Security — encryption, auth, trust)
                 └→ Story 8 (LLM security — trust levels, injection detection)
       └→ Story 7 (Token awareness — budgets, cursors, config)
  └→ Story 9 (CI/CD — can be built in parallel with most stories)
```

Story 1 is the acceptance test for "bpmux exists and is usable." Every subsequent story adds a vertical slice of value on top of it.

Story 9 (CI/CD) can start as early as Story 0 — you want CI from the first commit. It grows as more stories add more things to test.

Stories 2, 3, and 5 can potentially be parallelized (MCP, Web UI, and WebRTC are independent bounded contexts) but each depends on Story 1's session/log infrastructure.

---

## What These Stories Are NOT

- They are not final. They are candidates for discussion.
- They do not have estimates. Estimation happens during planning conversations.
- They do not prescribe implementation order. The sequencing above is suggested, not mandated.
- They are not in GitHub Issues yet. That happens after the human reviews and approves them.

These are starting points for the implementation plan conversation.
