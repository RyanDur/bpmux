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

## Acceptor rules (who accepts a story)

Acceptor-class follows story-TYPE — not re-derived or negotiated per story.

- **Chore → dev-authored + dev-accepted**, categorically. What makes a story a Chore is **no perceived outward _product_ behavior** (internal tooling/plumbing — even when it surfaces workflow-metadata like an acceptor label). The team still desk-checks it and gives verdicts, but **dev fires the accept**. The acceptor-axis below does NOT apply.
- **Feature / Bug → acceptor-axis** from the user-job's leading `As a [role]` clause: `As the owner` / `As Ryan` → **owner-accept** (owner-walk); a lone `As a dev` → **dev-accept**; `As a pm` / `As an agent` / `team` / multi-role (`dev`+`pm` together) → **team-accept**. PM picks the class from that clause. Beneficiary ≠ walker — the owner may _benefit_ (e.g. a board-render) while the _agent_ is the walker → team.

`derive_acceptor_label` reads only that leading `As a [role]` clause (set via `bpmux create/edit --acceptor`, voiced at the gate via `bpmux pass --acceptor`). **Run it on Features/Bugs only** — running the acceptor-axis on a Chore is a category-error; a Chore is always dev-accept regardless of role words in its body. (Owner-ruled 2026-06-22.)
