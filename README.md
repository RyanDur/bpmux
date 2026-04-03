# bpmux

P2P group chat for AI pair programming sessions.

bpmux connects everyone in a pairing session — both humans and both AIs — in a shared conversation. When one AI explains its reasoning, the other AI reads it. When a human types a message, both AIs see it. No context is lost between sessions.

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

This opens a TUI chat. Type messages and press Enter. Press Esc to quit. Messages persist — run it again and your conversation is there.

### Connect Claude Code

One-time setup:

```bash
claude mcp add --transport http bpmux http://127.0.0.1:4117/mcp
```

Then start bpmux in one terminal and Claude Code in another. Claude will have `send_message` and `read_messages` tools. You both see the same conversation.

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

## Development

```bash
cargo build              # Build
cargo test               # Run all tests
cargo clippy -- -D warnings  # Lint
cargo fmt --check        # Check formatting
```

## License

Business Source License 1.1. See [LICENSE](LICENSE).
