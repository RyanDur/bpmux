# bpmux architecture foundation

bpmux's architecture is based on **The App Continuum** — a 10-stage
ladder from monolithic flat structure to distributed microservices. The
Continuum lets a codebase DEFER the monolith-vs-microservices decision
until evidence accrues, progressing stage-by-stage as friction demands.

## Source

- **Canonical URL**: https://www.appcontinuum.io/
- Read it directly; the distillation below is bpmux's current-state
  map, not a replacement for the source. Each agent forming their own
  read is the intent — divergence between reads is signal, not friction.

## Current stage (verified 2026-04-18)

**bpmux is at v3 (feature-grouped / bounded-contexts) single-crate.**

Evidence:

- Single-crate workspace — `Cargo.toml` declares one `bpmux` package;
  no sub-crates.
- Top-level modules align with business capabilities, not horizontal
  layers: `bridge` (MCP stdio integration) / `mcp` (MCP server runtime)
  / `session` (persistent storage, lessons, identity) / `terminal`
  (TUI) / `transport` (P2P networking, protocols).
- Explicit module boundaries via `mod.rs` re-exports. Each feature
  group exposes a curated public API; internals stay private.
- No circular dependencies between feature groups. Dependency flows
  downward: `bridge` + `server` depend on `session` + `transport`;
  `terminal` depends on `session` only; `session` is a leaf.

**Approaching v4 (components-within-app) but not there.** v4 requires
independently-built-and-tested components with explicit declared
dependencies. bpmux modules are logically separate but test-wise
coupled (integration tests span features, spin up the full runtime).

## Current frictions

Ranked by pressure toward next-stage progression:

1. **`terminal/` deeply models `session/` types** — 42 terminal files
   directly import `crate::session`. Session schema changes require
   auditing terminal rendering. A v4 move would decouple via event
   streams or view-model layer.
2. **`bridge/` tool responders duplicate `session/` access logic** —
   no shared query layer. Adding an identity field requires touching
   `session/identity.rs`, `bridge/tools/get_identity.rs`, and bridge
   helpers. Coordination cost grows with change frequency.
3. **Integration tests require full runtime** — `tests/*_test.rs` spin
   up server + session + terminal together. No component-level test
   isolation. A v4 move would extract testable modules.

## Where v3 is working

The Continuum argues staying at a stage is often correct.

- **Feature-grouping maps to team mental model** — "I'm working on
  session awareness" translates directly to `src/session/awareness/`.
  No search overhead. Code structure matches domain language (DDD
  bounded contexts).
- **Transport decoupled from business logic** — WebRTC support added
  with zero changes to bridge or session. Transport is resilient to
  layer additions because boundaries were explicit.
- **Test harness mirrors module structure** — `tests/harness/` follows
  `bridge/`, `pty/`, `peer/`, `screen/`. Test intent matches code
  intent.

## Stage-progression recommendation

Stay at v3 for the next 2-3 stories; begin v4 preparation work in
parallel. The Continuum's guidance: progress when friction is urgent,
not speculatively.

Concrete first v4 step: extract a `bridge::view_model` boundary that
translates session types into bridge-owned types. Testable with mocked
session; decouples bridge from session refactor; unblocks friction #2
without committing the whole codebase to v4. One or two stories of
work, reversible.

bpmux likely doesn't need v6+ (microservices) at any foreseeable point
— it's a single coordination substrate, not a service mesh. The
Continuum's v3-v5 range is the load-bearing window for this product.

## Storage layer: Strategy-pattern DI (#1289)

bpmux's persistence is abstracted behind two traits, both at
`applications/bpmux/src/session/storage.rs`:

- **`Storage`** — session-scoped persistence. Owns reads/writes for
  state that lives under a session directory: messages
  (`messages-current.jsonl`), lessons + peer lessons, generic
  key-value state, board snapshot (`board-store.json`), outbox
  (`board-outbox.json`), bookmarks, pins, reactions, iteration
  markers.
- **`GlobalStore`** — cross-session persistence. Owns reads/writes
  for state at user-global paths under `~/.bpmux/` that survive
  multiple sessions: priority (`~/.bpmux/priority.json`) initially;
  identity / briefings / tokens / usage / auth-token migration
  deferred to #1290.

**Two production impls + two test impls**, each storage type has
contract tests proving FileStore / MemoryStore semantic-parity:

- `FileStorage` + `FileGlobalStore` — disk-backed (production).
  Atomic-rename + fsync semantics stay inside FileStorage; tests'
  knowledge of those idiosyncrasies is zero.
- `MemoryStorage` + `MemoryGlobalStore` — in-memory (tests).
  Per-instance `Mutex<T>` state; no I/O. Selected via
  `BPMUX_STORAGE_BACKEND=memory` env-var at `Session::new`
  construction time.

**Production behavior preserved by path-injection builders** —
each FileStorage path is `Option<PathBuf>` with builder
`with_<thing>_path(p)`. Default (None) resolves via the existing
`default_<thing>_path()` helpers (env-overridable, e.g.,
`BPMUX_BOARD_STORE_PATH` + `BPMUX_BOARD_OUTBOX_PATH`; home-default
fallback). Production constructs with None; tests construct with
explicit per-test tempdirs to avoid sharing `~/.bpmux/*` files.

**Contract-test pattern at
`applications/bpmux/src/session/storage_tests.rs`** — `contract!`
macro runs the same assertion against both `FileStorage` and
`MemoryStorage` (sister `global_contract!` for the GlobalStore
pair). Behavioral parity is verified at test-write time; divergence
is impossible to land. 67 contract tests covering 10 session-scoped
types + priority.

**Stage-fit**: this refactor strengthens v3 internal-bounded-context
boundaries (session-storage capability has a clean DI seam) without
forcing v4 components-within-app. Sister-pattern reference: the
existing `BoardStore` + `InMemorySourceAdapter` at the SourceAdapter
layer predates and validates this shape.

**Test-substrate benefit + open follow-up**: tests injecting
`MemoryStorage` skip per-test backend-startup-disk-I/O cost — the
substrate-class that empirically falsified macOS self-hosted e2e
migration on f9ebe47 / c8ba878 / 5bdb235. Full wall-clock
measurement + launch-helper test-seed-API + remaining Phase-3
globals (identity / briefings / tokens / usage / auth-token) ship in
#1290.

## Why this doc exists

bpmux's architectural substrate kept getting re-discovered + lost
across compactions. Per the "re-derivation signals externalization"
discipline, team-load-bearing context belongs in version-controlled
docs, not per-role auto-memory. This doc is the architectural ground
truth; individual roles' reads of the Continuum stay in their own kb.

## Cross-reference

- `memory-architecture.md` — how arcs / lessons / principles / values
  relate; pairs with this doc at session-start
- role-specific distillations live in per-workspace kb/:
  `/Users/ryandurling/IdeaProjects/bpmux-dev/kb/references/reference_app_continuum.md`
  + equivalents for pm/designer
