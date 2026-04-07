# bpmux

P2P group chat for AI pair programming sessions. Prevents cognitive and intent debt by capturing AI reasoning in a persistent, transparent shared environment.

## Commands

```bash
cargo build              # Build
cargo test               # Run all tests
cargo clippy -- -D warnings  # Lint (warnings are errors)
cargo fmt --check        # Check formatting
cargo run                # Run bpmux
```

## Architecture

Organized by bounded context (App Continuum), not by technical layer:

- `src/session/` — Session state, message log, JSONL persistence
- `src/terminal/` — TUI rendering, input handling, chat display
- `src/mcp/` — MCP tool handler (send/read messages via MCP protocol)
- `src/transport/` — HTTP/WebSocket transport layer (REST API, WebSocket protocol, shared server state)
- `src/bridge/` — Channel bridge: stdio MCP server connecting Claude Code to bpmux via WebSocket
- `src/server.rs` — HTTP server startup, router assembly, graceful shutdown

## Code Style

- **Error handling:** `Result<T, E>` everywhere. No `unwrap()` or `expect()` in production. `thiserror` for custom errors. `?` for propagation.
- **Immutability by default.** Only use `mut` when necessary.
- **Iterators over loops.** Use `.map()`, `.filter()`, `.collect()`.
- **No `unsafe` code.** `#![deny(unsafe_code)]` at crate root.
- **Tests as documentation.** Test names describe behavior: `sends_message_to_log`, not `test_send`.
- **Comments for why, not what.** If code needs a comment to explain what it does, refactor.

## Testing

- TDD: outside-in, classicist (real collaborators over mocks)
- Unit tests in separate `_tests.rs` files next to the source (e.g. `app.rs` → `app_tests.rs`), declared via `#[cfg(test)] mod x_tests;` in the parent `mod.rs`
- Integration tests in `tests/`
- Tests exercise the public API, not private internals
- `cargo test` must pass before push

## Working Agreement

See `AGENT_WORKFLOW.md` for how we collaborate. It is our process, not a bpmux feature.
