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

## Code Style

- **Error handling:** `Result<T, E>` everywhere. No `unwrap()` or `expect()` in production. `thiserror` for custom errors. `?` for propagation.
- **Immutability by default.** Only use `mut` when necessary.
- **Iterators over loops.** Use `.map()`, `.filter()`, `.collect()`.
- **No `unsafe` code.** `#![deny(unsafe_code)]` at crate root.
- **Tests as documentation.** Test names describe behavior: `sends_message_to_log`, not `test_send`.
- **Comments for why, not what.** If code needs a comment to explain what it does, refactor.

## Testing

- TDD: outside-in, classicist (real collaborators over mocks)
- Unit tests in same file (`#[cfg(test)]` module)
- Integration tests in `tests/`
- `cargo test` must pass before push

## Working Agreement

See `AGENT_WORKFLOW.md` for how we collaborate. It is our process, not a bpmux feature.
