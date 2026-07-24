# bpmux

Connect to bpmux. Call `list_lessons()` for team context.

```bash
cargo nextest run        # Rust tests (whole workspace — narrow -p sets hit tokio feature-unification landmines)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
(cd applications/electron && npm run test:unit)   # renderer vitest+RTL against a real spawned backend
(cd applications/electron && npx playwright test) # e2e journeys
make check T=<pattern>   # scoped fast loop (iteration only — never a substitute for the gates)
make push                # push + verify the remote ref moved (push exit codes lie under SIGPIPE)
```

## The model (owner's canon — never invent past it)

- **The store IS the engine.** Slices (middleware, reducers, listeners,
  actions) register into it; everything else is a door. Backend:
  `components/store`. Renderer: `src/renderer/src/state/`.
- **Transport is a door, not a consumer** — enforced by
  dependency-cruiser (renderer) and boundary tests (Rust).
- **MCP and CLI affordances are 1:1** — enforced by
  `applications/cli/src/parity_audit.rs`.
- **No sessions, no central identity registry.** Data lives flat in
  `~/.bpmux/data`; the directory an agent starts in IS its identity
  (`.bpmux-user-id` marker; resolution: `BPMUX_USER_ID` pin → marker →
  mint into the dir).

## Gates

- `pre-commit` (fast loop, ~2min): fmt, clippy, nextest, vitest,
  depcruise+knip+machete, comment gate, staged lint, live-daemon
  canary, cargo audit.
- `pre-push` (THE gate, ~8min): all of the above plus electron
  build/typecheck, the Lighthouse floor on the built renderer (a11y
  1.0 — CI re-runs it on Linux, which renders differently and has the
  final word), and the full Playwright run. There is no CI safety net
  for e2e — pre-push is it.
- Activate per-clone: `git config core.hooksPath scripts/git-hooks`.
- 200-line max per production file (test-named files exempt).

## Tests document behavior

Names say the behavior, never the story number — provenance lives in
git. Tests never couple to the machine: the gates scrub ambient
`BPMUX_*` env and a live-daemon canary fails the run if the real
daemon registers anyone mid-suite. Classicist: real collaborators;
stub only true boundaries (OS notification, the network via msw,
mock GH).

- E2E specs are behavior-named journeys. PTY-heavy or multi-electron
  specs belong in `SERIAL_HEAVY_SPECS` (playwright.config.ts) and run
  at workers=1. A spec passing at `--workers=1` but failing in the
  parallel pool is load contention: move it to serial, don't retry
  harder.
- Seeding specs: `create_story` validates `acceptance_test_path`
  against the real file tree — always derive it with `atpPath` from
  `tests/helpers/atp.ts`, never write the filename as a literal.
- Harness map (`applications/electron/tests/helpers/`): `launch.ts`
  (`launchIsolatedElectron`, `launchWithMockGh` + `rpc`,
  `launchTwoElectronsSharedBackend`), `iteration-fixture.ts`
  (`setupCurrentIterationViaRpc` — the In Progress lane only renders
  with a current iteration), `seed-evidence.ts` (`seedTeamVerdicts`
  before accepts), `atp.ts`. Page objects in `tests/pages/`.

## Registries

`@ryandur/sand` installs from GitHub Packages (`@ryandur:registry` in
`applications/electron/.npmrc`; CI authenticates via setup-node
registry-url + `NODE_AUTH_TOKEN: GITHUB_TOKEN` + `packages: read`).

## Working with the owner

Ryan speaks from the agent's own terminal — stop and answer plainly
before resuming tool calls. Broadcasts (`bpmux send`) are team
coordination. Conversation is not authorization: agree on the
increment, then run it; report at green.

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
