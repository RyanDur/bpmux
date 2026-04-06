# bpmux

Collective memory for human-AI teams.

bpmux is a shared message bus that makes AI reasoning transparent, persistent, and reviewable. When AI agents explain their thinking, everyone sees it. When decisions are made, the rationale is captured. No context is lost between sessions.

Built to prevent cognitive debt (eroded shared understanding) and intent debt (absent rationale) in AI-assisted development.

## Quick Start

### Build

```bash
git clone git@github.com:RyanDur/bpmux.git
cd bpmux
cargo build
```

### Run

```bash
cargo run -- --name ryan
```

Opens a TUI chat. Type messages, press Enter to send. Messages persist across restarts.

### Desktop Notifications (recommended)

Install [code-notify](https://github.com/mylee04/code-notify) to get macOS desktop notifications when agents finish tasks or need permission:

```bash
brew install mylee04/tools/code-notify
cn on claude
cn test  # verify you see a notification
```

Without code-notify, you won't know when agents are blocked unless you're watching the TUI.

### Connect Claude Code (MCP)

Add bpmux as an MCP server:

```bash
claude mcp add --transport http bpmux http://127.0.0.1:4117/mcp
```

Start bpmux in one terminal, Claude Code in another. Claude gets `send_message` and `read_messages` tools. Both see the same conversation.

### Connect via Bridge (real-time channels)

The bridge connects Claude Code to bpmux via WebSocket for real-time message flow. Create a `.mcp.json` in your project root:

```json
{
  "mcpServers": {
    "bpmux-bridge": {
      "type": "stdio",
      "command": "cargo",
      "args": ["run", "--manifest-path", "/path/to/bpmux/Cargo.toml", "--", "bridge", "--port", "4117", "--name", "your-name"]
    }
  }
}
```

Replace `/path/to/bpmux/Cargo.toml` with your actual path. The `--name` flag sets your identity in the chat.

Launch Claude Code with the bridge:

```bash
claude --dangerously-load-development-channels server:bpmux-bridge
```

### Connect Other AI Tools

Any MCP client that supports HTTP transport can connect to:

```
http://127.0.0.1:4117/mcp
```

## Options

```
--name <NAME>          Your name in the session (default: dev)
--mcp-port <PORT>      HTTP/MCP server port (default: 4117)
--session-base <PATH>  Session directory (default: ~/.bpmux/sessions)
```

## Directed Messaging

Send messages to specific participants:

```
/to dev hello                   → DM to dev
/to dev,designer let's discuss  → group message to dev + designer
```

Messages without `/to` broadcast to everyone. The TUI tags each message with its channel: `[broadcast]`, `[you↔dev]`, `[dev+designer+pm]`.

Tab completion works after `/to` — type a few characters and press Tab.

## TUI Keybindings

| Key | Action |
|-----|--------|
| Enter | Send message |
| Ctrl+U | Scroll up (half page) |
| Ctrl+D | Scroll down (half page) |
| PageUp/PageDown | Scroll up/down |
| Ctrl+T | Toggle timestamps |
| Tab | Complete recipient name after `/to` |
| Esc | Quit |
| /quit | Quit |

## Team Development Setup

bpmux uses a hub-and-spoke workspace model. The shared repo (`bpmux/`) holds all code, docs, and team norms. Each AI role gets its own Claude Code workspace (`bpmux-dev/`, `bpmux-pm/`, `bpmux-designer/`) with its own `.mcp.json` and `CLAUDE.md`.

**Why separate workspaces?** Each AI agent accumulates context and memory specific to its role — dev learns about architecture, PM learns about backlog priorities, designer learns about UX conventions. Separate workspaces keep this context naturally segregated. Anything that needs to be shared lives in the repo or in the bpmux chat log, not in any single agent's memory.

### Directory structure

```
IdeaProjects/
├── bpmux/              # Shared repo — code, docs, roles, norms
├── bpmux-dev/          # Dev workspace (Claude Code)
│   ├── .mcp.json       # Bridge connection config
│   └── CLAUDE.md       # Dev-specific instructions
├── bpmux-pm/           # PM workspace (Claude Code)
│   ├── .mcp.json
│   └── CLAUDE.md
└── bpmux-designer/     # Designer workspace (Claude Code)
    ├── .mcp.json
    └── CLAUDE.md
```

### Setting up a new role workspace

1. Create the workspace directory next to `bpmux/`:
   ```bash
   mkdir bpmux-{role}
   ```

2. Add a `.mcp.json` that connects to the bridge:
   ```json
   {
     "mcpServers": {
       "bpmux-bridge": {
         "type": "stdio",
         "command": "cargo",
         "args": ["run", "--manifest-path", "../bpmux/Cargo.toml", "--", "bridge", "--port", "4200", "--name", "your-role"]
       }
     }
   }
   ```

3. Add a `CLAUDE.md` that points to the shared repo:
   ```markdown
   # bpmux-{role} workspace
   Read these first:
   1. `../bpmux/welcome.md` — team values and norms
   2. `../bpmux/roles/{role}.md` — your role contract
   3. `../bpmux/AGENT_WORKFLOW.md` — how we work
   ```

4. Launch Claude Code in the workspace with the bridge:
   ```bash
   cd bpmux-{role}
   claude --dangerously-load-development-channels server:bpmux-bridge
   ```

### Running a team session

1. Start bpmux in one terminal:
   ```bash
   cd bpmux && cargo run -- --name ryan
   ```

2. Launch each agent in its own terminal:
   ```bash
   cd bpmux-dev && claude --dangerously-load-development-channels server:bpmux-bridge
   cd bpmux-pm && claude --dangerously-load-development-channels server:bpmux-bridge
   cd bpmux-designer && claude --dangerously-load-development-channels server:bpmux-bridge
   ```

3. All participants see each other in the TUI. Messages persist in the JSONL log.

## Claude Code Plugin

The `claude-plugin/` directory contains hooks for automatic bpmux integration:

- **UserPromptSubmit hook** — injects recent bpmux messages before each Claude response
- **Stop hook** — checks for new messages after Claude responds, continues the conversation if new messages arrived
- **Bridge MCP server** — connects Claude Code to bpmux via WebSocket

To use the plugin:

```bash
claude --plugin-dir ./claude-plugin --dangerously-load-development-channels server:bpmux-bridge
```

## Migrating to a New Machine

Session data lives in `~/.bpmux/` (not in the repo). To transfer everything to a new machine:

1. **Push all commits** from the old machine
2. **Clone the repo** on the new machine
3. **Copy session data:**
   ```bash
   # On the old machine
   cp -r ~/.bpmux ~/transfer-bpmux

   # On the new machine
   cp -r ~/transfer-bpmux ~/.bpmux
   ```
4. **Copy workspace directories** (`bpmux-dev/`, `bpmux-pm/`, `bpmux-designer/`) — `.mcp.json` uses relative paths, so no edits needed if the sibling layout is preserved
5. **Reinstall dependencies:**
   ```bash
   # Rust toolchain
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Desktop notifications
   brew install mylee04/tools/code-notify
   cn on claude
   ```

### Session directory structure

```
~/.bpmux/
└── sessions/
    ├── current -> {uuid}/          # Symlink to active session
    └── {uuid}/
        ├── messages-current.jsonl  # Chat history (the primitive)
        └── pins.json               # Pinned messages
```

The JSONL log is append-only. Each line is a self-contained message with sender, timestamp, channel, and content. Sessions are identified by UUID — the `current` symlink points to the active one.

## Architecture

Organized by bounded context (App Continuum). Each directory owns one concern — no cross-boundary imports except through explicit interfaces.

**Why bounded contexts?** bpmux is a message bus with multiple integration surfaces (TUI, HTTP/MCP, WebSocket, stdio bridge). Organizing by technical layer (models/, views/, controllers/) would scatter each integration across the entire tree. Bounded contexts keep each integration self-contained, so changes to the MCP interface don't touch the TUI.

```
src/session/    — Session state, message log, JSONL persistence
src/terminal/   — TUI rendering, input handling, chat display
src/mcp/        — MCP tool handler (send/read messages via MCP protocol)
src/transport/  — HTTP/WebSocket transport layer (REST API, WebSocket protocol)
src/bridge/     — Channel bridge: stdio MCP server connecting AI tools to bpmux
src/server.rs   — HTTP server startup, router assembly
src/lib.rs      — CLI parsing + orchestration
```

## Development

```bash
cargo build              # Build
cargo test               # Run all tests
cargo clippy -- -D warnings  # Lint (warnings are errors)
cargo fmt --check        # Check formatting
```

See [CLAUDE.md](CLAUDE.md) for code style and conventions. See [AGENT_WORKFLOW.md](AGENT_WORKFLOW.md) for how the team collaborates.

## Why bpmux?

AI coding tools make individuals faster but erode team coordination. Code gets generated faster than teams can comprehend it. Design rationale vanishes when AI sessions end. Multiple agents editing the same codebase creates coordination chaos.

bpmux addresses this by making the conversation — the reasoning, the decisions, the tradeoffs — a persistent, shared, reviewable artifact. The JSONL log is the primitive. Everything else is built on top of it.

See [docs/research/synthesis/thesis-v2.md](docs/research/synthesis/thesis-v2.md) for the full thesis.

## License

Business Source License 1.1. See [LICENSE](LICENSE).
