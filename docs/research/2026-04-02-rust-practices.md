# Rust Practices for bpmux

*Compiled from Apollo GraphQL's Rust Best Practices, Mayhem Security's Secure Rust guide, and bpmux's own design constraints.*

---

## Error Handling (aligns with our ROP / railway-oriented approach)

- **`Result<T, E>` everywhere.** No panics in production paths.
- **`thiserror` for library-level errors.** Custom error types per bounded context.
- **`?` operator for propagation.** Railway-style early returns.
- **No `unwrap()` or `expect()` in production.** Already in spec via clippy lints (`clippy::unwrap_used`, `clippy::expect_used` = deny).
- **`anyhow` for the binary entrypoint only.** `main.rs` can use `anyhow::Result` for top-level error reporting. Bounded contexts use `thiserror`.
- **Test error paths.** Every `Result`-returning function should have a test that exercises the `Err` case.

## Testing (aligns with our classicist TDD approach)

- **Tests as living documentation.** Test names describe behavior, not implementation.
- **Doc tests on public APIs.** Executable examples in `///` comments.
- **Snapshot testing with `cargo-insta`** for complex outputs (TUI rendering, message formatting).
- **Distinguish test types by purpose:** unit (per bounded context), integration (cross-context), e2e (real binary).
- **Real collaborators over mocks.** Use real implementations wherever feasible (already in spec).

## Code Organization

- **Borrowing over cloning.** Prefer `&T` and `&mut T`. Clone only when ownership transfer is needed.
- **When to pass by value:** small `Copy` types (integers, booleans, small structs with `#[derive(Copy, Clone)]`).
- **Iterators over explicit loops.** Use `.iter()`, `.map()`, `.filter()`, `.collect()`. Zero-cost abstractions.
- **Prevent early allocation.** Delay `Vec::new()`, `String::new()` until you know you need them. Use iterators to compose without intermediate allocations.
- **Organize imports clearly.** Group by: std, external crates, internal modules. One blank line between groups.

## Safety

- **`#![deny(unsafe_code)]` at crate root.** Already in spec. No `unsafe` in bpmux code.
- **Minimize `unsafe` in dependencies.** Prefer pure-Rust crates (str0m over datachannel-rs FFI).
- **Sanitize all input.** Messages from MCP clients, WebRTC peers, and the control socket are untrusted input. Validate before processing.
- **Use the type system to enforce invariants.** Newtype wrappers for session IDs, peer names, message content. Prevent stringly-typed bugs.
- **Type State Pattern** for connection lifecycle: `Disconnected → Connecting → Connected → Degraded`. Invalid state transitions are compile-time errors.

## Performance

- **Don't optimize prematurely.** Profile first with `cargo flamegraph`.
- **Avoid redundant cloning.** `Arc<T>` for shared ownership across tokio tasks. Clone the `Arc`, not the data.
- **Stack vs heap.** Small structs on the stack. Large or dynamically-sized data on the heap (`Box`, `Vec`, `String`).
- **Async overhead is real.** Don't make every function async. Only async at I/O boundaries.

## Cryptography

- **Use audited crates.** `ring` or `aes-gcm` + `pbkdf2` for at-rest encryption. Already in spec.
- **Don't roll your own.** The spec uses PBKDF2-HMAC-SHA256 (600k iterations, 16-byte salt) and AES-256-GCM — standard, well-understood constructions.
- **Review cryptographic code carefully.** Crypto is easy to implement incorrectly even with good crates.

## Code Quality

- **`cargo clippy` with deny warnings.** Already in spec: `cargo clippy -- -D warnings`.
- **Fix warnings, don't silence them.** If a warning is wrong, understand why before `#[allow]`.
- **Comments for why, not what.** Already in working agreement: "If code needs a comment to explain what it does, refactor."
- **`cargo fmt` enforced.** Already in spec: pre-push hook runs `cargo fmt --check`.
- **`cargo audit` + `cargo deny` in CI.** Already in spec.

## bpmux-Specific Patterns

- **Bounded contexts as modules.** `signaling/`, `transport/`, `session/`, `mcp/`, `terminal/`, `web/` — each with its own error types, public API, and internal implementation.
- **Message bus via `tokio::sync::broadcast`.** Wake signals only, not payloads. Consumers read from the log.
- **`Arc<Mutex<T>>` only for session state.** Prefer owned data and message passing elsewhere. Immutability by default.
- **Feature flags in Cargo.toml for optional components.** The signaling server (`bpmux serve`) doesn't need `ratatui`. The headless mode doesn't need `crossterm`.

## Sources

- [Apollo GraphQL — Rust Best Practices](https://github.com/apollographql/rust-best-practices)
- [Mayhem Security — Secure Programming in Rust](https://www.mayhem.security/blog/best-practices-for-secure-programming-in-rust)
