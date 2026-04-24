# bpmux

Connect to bpmux. Call `list_lessons()` for team context.

```bash
cargo build              # Build
cargo test               # Run all tests
cargo clippy -- -D warnings  # Lint (warnings are errors)
cargo fmt --check        # Check formatting
cargo run                # Run bpmux
```

## Team principles (apply per-commit, every story)

These are team-discipline, not stories. Every new commit satisfies them. Refactor adjacent code that violates them as you touch a file. See debt, pay debt — do not wait.

1. **Arrow functions only.** No `function` declarations. Enforced via eslint. Exception: React class components required for error-boundary lifecycle methods.
2. **No refs as first reach.** `useRef` signals "fighting React" — prefer CSS/layout/declarative state (flex, grid, sticky, column-reverse, `@starting-style`). Refs legit only for form DOM integration or 3rd-party lib handles.
3. **No comments.** Well-named code + commit messages communicate. Comments rot; structure doesn't. If the artifact needs prose to justify itself, the shape is wrong — restructure.
4. **Railway-oriented programming on front and back.** Expected failures go in the type system as `Result<T, E>` / `Either` / `Maybe`. Not in exception flow. Every function signature shows what can fail; every caller handles the failure track explicitly.
5. **Schema validation at boundaries.** Decoders on all incoming data — WS messages, HTTP responses, MCP payloads, IPC. Never `as T` a raw JSON. Parse-don't-validate: the decoder IS the contract; invalid data fails loud at the edge, not silently inside.

**Refactor as you go.** Every commit leaves its file better than it found it. No dedicated refactor sprints, no "migrate X" stories, no waiting for the right time. The time is now.

**Architecture reference:** battleship + Ryan's public repos (sand, ChosenPicachu, Deaf-Beavers) — Result/Maybe/AsyncResult via sand; schemawax decoders; ports-adapters with parallel fakes; feature-grouped source; CSS split by concern (`*.css` paint + `*.layout.css` structure).
