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

## Functional Programming in Rust (aligns with your Kotlin/TypeScript FP experience)

Rust is "aggressively mixed-paradigm" — FP is native, not bolted on. Your existing FP muscle memory transfers directly:

**What transfers from your TypeScript/Kotlin ROP style:**
- `Result<T, E>` with `map`, `and_then`, `or_else`, `unwrap_or` → your `Result.map`, `andThen`, `or`, `either`
- `Option<T>` with the same combinators → your `Maybe` type
- `?` operator for early returns → your railway-oriented pipelines
- `enum` with `match` → your sealed classes / discriminated unions
- Closures as first-class values → your arrow functions
- `Iterator` trait with `.map()`, `.filter()`, `.fold()` → your functional collection pipelines

**Core FP practices for bpmux:**
- **Prefer iterators over manual loops.** Rust iterators are lazy (execute only when consumed) and the compiler optimizes them to be as fast as manual `for` loops. Use `.map()`, `.filter()`, `.collect()` for declarative data pipelines.
- **Immutability by default.** Rust enforces this — variables are immutable unless you write `mut`. This aligns with the spec's "immutability by default" principle and with your existing style.
- **Algebraic data types (enums) + exhaustive pattern matching.** Use `enum` and `match` instead of inheritance. The compiler ensures every variant is handled. This replaces your Kotlin sealed classes.
- **Pure functions where possible.** Output depends solely on inputs, no side effects. More testable, more predictable.
- **Higher-order functions.** Functions that take functions as arguments (`.map()`, `.filter()`, `.fold()`). Same patterns as your TypeScript/Kotlin code.

**Cautions:**
- **Avoid excessive recursion.** Rust does not guarantee tail-call optimization (TCO). For large datasets, use iterators instead. This is different from languages with TCO (Haskell, Scala).
- **Pragmatism over purity.** Rust is not Haskell. If a nested iterator chain is harder to read than a simple loop, use the loop. "Idiomatic Rust is whatever is easiest to read and maintain."
- **Ownership complicates closures.** Closures that capture variables by reference interact with the borrow checker. Use `move` closures when spawning tokio tasks. This is the main Rust-specific learning curve for FP.

**Sources:** [ANSSI Secure Rust Guidelines](https://anssi-fr.github.io/rust-guide/) | [Google Comprehensive Rust](https://google.github.io/comprehensive-rust/)

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

## Project Structure

- **Thin binary.** `src/main.rs` is a thin wrapper that creates the tokio runtime, parses CLI args, and dispatches to `src/lib.rs`. All business logic lives in `src/lib.rs` and its modules. This makes bounded contexts testable without running the full binary.
- **`pub use` re-exports.** Each bounded context (`signaling/`, `transport/`, `session/`, etc.) has a `mod.rs` or module file that re-exports its public API. Internal structure is hidden. External callers see a flat API.
- **`Cargo.lock` committed.** bpmux is a binary, not a library. The lock file ensures reproducible builds and goes in git.
- **`cargo check` for fast feedback.** Faster than `cargo build` — checks types and borrows without codegen. Use during development for quick iteration.

## Naming Conventions (Rust API Guidelines)

- **Types, Traits, Enums:** `PascalCase` — `SessionState`, `SignalingMessage`, `TrustLevel`
- **Functions, Variables, Modules:** `snake_case` — `read_messages`, `session_code`, `signaling`
- **Constants, Statics:** `SCREAMING_SNAKE_CASE` — `DEFAULT_MCP_PORT`, `MAX_MESSAGE_SIZE`
- **Enum variants:** `PascalCase` — `TrustLevel::PeerAi`, `SessionState::Degraded`
- **Crate/module names:** `snake_case`, no hyphens in module names (hyphens OK in Cargo.toml package name)

## bpmux-Specific Patterns

- **Bounded contexts as modules.** `signaling/`, `transport/`, `session/`, `mcp/`, `terminal/`, `web/` — each with its own error types, public API, and internal implementation.
- **Message bus via `tokio::sync::broadcast`.** Wake signals only, not payloads. Consumers read from the log.
- **`Arc<Mutex<T>>` only for session state.** Prefer owned data and message passing elsewhere. Immutability by default.
- **Feature flags in Cargo.toml for optional components.** The signaling server (`bpmux serve`) doesn't need `ratatui`. The headless mode doesn't need `crossterm`.

## Sources

- [Apollo GraphQL — Rust Best Practices](https://github.com/apollographql/rust-best-practices)
- [Mayhem Security — Secure Programming in Rust](https://www.mayhem.security/blog/best-practices-for-secure-programming-in-rust)
