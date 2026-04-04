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

## TUI Keybindings

| Key | Action |
|-----|--------|
| Enter | Send message |
| Ctrl+U | Scroll up (half page) |
| Ctrl+D | Scroll down (half page) |
| PageUp/PageDown | Scroll up/down |
| Ctrl+T | Toggle timestamps |
| Esc | Quit |
| /quit | Quit |

## Claude Code Plugin

The `claude-plugin/` directory contains hooks for automatic bpmux integration:

- **UserPromptSubmit hook** — injects recent bpmux messages before each Claude response
- **Stop hook** — checks for new messages after Claude responds, continues the conversation if new messages arrived
- **Bridge MCP server** — connects Claude Code to bpmux via WebSocket

To use the plugin:

```bash
claude --plugin-dir ./claude-plugin --dangerously-load-development-channels server:bpmux-bridge
```

## Architecture

Organized by bounded context (App Continuum):

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
