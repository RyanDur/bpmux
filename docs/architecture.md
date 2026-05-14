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
  multiple sessions: priority (`~/.bpmux/priority.json`), per-role
  identity (`~/.bpmux/identity/{role}.json`), per-role briefing
  (`~/.bpmux/briefings/{role}.md`), token log
  (`~/.bpmux/tokens/log.jsonl`), feature-usage counters
  (`~/.bpmux/usage.json`), and per-run auth token
  (`~/.bpmux/auth_token`).

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
is impossible to land. 94 contract tests covering 10 session-scoped
types + 5 cross-session globals (priority / identity / briefing /
tokens / usage). One global — `auth_token` — has a **divergent
contract** by boundary-discipline: `FileGlobalStore` wraps mode-0600
filesystem semantics; `MemoryGlobalStore` returns
`ErrorKind::Unsupported` because a process-owned cryptographic
secret has no in-memory equivalent. Tests requiring auth-token
isolation use `FileGlobalStore` with per-test
`BPMUX_AUTH_TOKEN_PATH` tempdir (established at
`applications/electron/tests/helpers/launch.ts`).

**Stage-fit**: this refactor strengthens v3 internal-bounded-context
boundaries (session-storage capability has a clean DI seam) without
forcing v4 components-within-app. Sister-pattern reference: the
existing `BoardStore` + `InMemorySourceAdapter` at the SourceAdapter
layer predates and validates this shape.

**Test-substrate benefit + open follow-up**: tests injecting
`MemoryStorage` skip per-test backend-startup-disk-I/O cost — the
substrate-class that empirically falsified macOS self-hosted e2e
migration on f9ebe47 / c8ba878 / 5bdb235. Phase-3 GlobalStore
extension (identity / briefings / tokens / usage / auth_token) shipped
under #1290 (sister-shape commits 2a91ccb / 1222506 / 9b2fc9d /
e8e5aaf / 49e3f94). Full wall-clock measurement + launch-helper
test-seed-API (Part A/B) complete #1290.

## Iteration substrate: canonical-via-GH issue (Epic #1189)

bpmux's iteration entities migrated from a local-only substrate
(`session/iteration_markers.json` markers-store + `session/priority.json`
icebox_unified positional manipulation) to a canonical-via-GH substrate
where iterations are first-class GitHub issues with `story_type=Iteration`
and `iteration_id` foreign-keys on member-stories. Epic #1189 sequenced
the architectural-pivot across six slices (#1297-#1302) using
strangler-fig parallel-substrate transition.

**User-job (architecture-agnostic)**: iterations group stories for
batched-by-theme planning, accept-walk ceremony, retro lineage, and
"what's next" prioritization. The user-job hasn't changed; the
architecture-fit underneath has.

**Architectural-pivot motivation**: today's substrate-Bug class
(#1283 missing-marker-restore + #1288 fold-position-loss + #1294
legacy stories[]-backfill + #1295 mid-session save_priority null-write
+ #1273 iteration-scoped guard) all manifested as priority.json /
markers-store consistency failures. Canonical-via-GH-issue substrate
eliminates the class: GitHub issues have built-in atomicity for state
transitions + label mutations + comment-history + close semantics +
cross-session persistence + multi-agent visibility. Sister-pattern to
#1289 Storage-trait DI at the iteration-substrate axis — both
ratifications strengthen v3 internal-bounded-context boundaries by
relocating capability state behind a canonical seam.

**Slice progression**:

| Slice | Issue | Surface |
|---|---|---|
| 1 | #1297 | `StoryType::Iteration` variant + `create_iteration(theme_name)` MCP tool + 'iteration' label decoder + TS schema parity |
| 2 | #1298 | `Story.iteration_id: Option<u64>` + `iteration_id_from_labels` decoder + `list_iteration_children(iteration_number)` tool + `iteration-#NNNN` label-emit + `get_story` surfaces iteration_id |
| 3 | #1299 | Membership tools (`add_stories_to_iteration` / `remove_stories_from_iteration`) fire iteration_id-via-label canonical chain alongside legacy icebox_unified manipulation (parallel substrate); `BoardStore::apply` TagAdded/TagRemoved sync raw_labels + label-derived fields |
| 4 | #1300 | `close_iteration(iteration_number)` canonical-via-GH ceremony refactor: closes GH iteration-issue + promotes members via canonical mutation chain; legacy markers-store path preserved for backward-compat; `CloseIterationKind` dispatcher pins routing |
| 5 | #1301 | One-shot manual migration recipe (no boot-time pipeline): PM reads `iteration_markers.json`, fires `create_iteration(theme_name)` for each active iteration, then `add_stories_to_iteration(iteration_number, story_ids)` for the members. Per Ryan 2026-05-12 22:54 UTC ("we shouldn't need it after this... just look at whats in the json and use the functions"): one-shot ceremonies don't need automated pipelines — imperative-fire-over-pipeline. Earlier auto-pipeline shipped at commits c316a87 + 4e4e610 was retired in this slice. |
| 6 | #1302 | Markers-store + icebox_unified marker-substrate retirement (close-out) — shipped via 9-commit vertical-slice chain: Phase 1 (MCP tool schemas) + Slices A/B/C/D (per-feature verticals: create / delete / reorder / read+renderer) + E.1 (close_iteration legacy path) + E.2 (add_stories markers coupling) + E.3 (remove_stories markers coupling) + E.4+E.5 (IceboxItem::Marker schema variant + session::iteration_markers module). ~4000 lines net deleted. Canonical-via-GH (story_type=Iteration + iteration_id label) is now sole substrate. Tolerant deserializer in priority.rs filters legacy `{kind:marker}` entries from existing priority.json on read for one-shot migration. **Renderer-grouping rework** (post-PASS): iteration-issues render as parent-rows in a sister-to-Epic `Iteration.tsx` component with member-stories grouped via `MiniBoard` (active membership decoded via `iteration_id` label-derivation + `isActiveIteration` filter; members excluded from flat-state columns so grouping is unambiguous). Self-reference guard added at `post_add_stories_to_iteration` (400 on `iteration_number ∈ story_ids`). Live-walk RTL+real-backend gate at `iteration-grouping-icebox.test.tsx` pins user-visible grouped-render + substrate self-reference rejection. |

**Strangler-fig discipline**: slices 3 + 4 + 5 shipped the NEW
canonical substrate ALONGSIDE the legacy markers-store substrate
during the transition window. The renderer read icebox_unified for
marker-grouping until Slice D retired the renderer-side hook +
components and the icebox column transitioned to flat-list rendering.
Slice 6 then retired the legacy substrate atomically — each
sub-slice retired one feature end-to-end across all layers
(consumer-side before producer-side, per vertical-slice canon). This
pattern matches the Storage-trait DI v3-strengthening shape: parallel
paths during transition, atomic close-out at the retirement seam.

**Chokepoint guarantee**: `BoardStore::apply` is the canonical
mutation-chain chokepoint (#1282 fire-ordering invariant). All eight
`BoardEvent::StateTransitioned` dispatch sites + all `TagAdded` /
`TagRemoved` / `StoryClosed` / `StoryCreated` / `LabelsRefreshed`
routes pass through it; the iteration-substrate inherits the
fire-ordering + label-decoder + iteration_id-sync guarantees
automatically. Sister to #1289 contract-test pattern that pins
FileStorage / MemoryStorage parity at write-time.

**Test-seed-API gap-list (Epic #1247 audit-output)**: the parallel-
RTL migration audit (#1305) surfaced 6 missing test-seed-APIs gating
bulk-migration batches 4.3-4.5 — `create_iteration_marker`,
`add_stories_to_iteration_marker`, `seed_messages`, `seed_unfurls`,
`seed_reactions`, `seed_pins`. The iteration-substrate pivot (Epic
#1189) is largely complete in advance of these test-seed-APIs landing;
the RTL-migration batches build the missing seed-APIs incrementally
per their batch-scope.

**Stage-fit**: this pivot strengthens v3 internal-bounded-context
boundaries (iteration-capability has a canonical-via-GH seam) without
forcing v4 components-within-app. Pre-pivot, iteration-state was
fragmented across two file-system substrates with no clean seam; post-
pivot, the canonical-via-GH-issue substrate is the seam.

## Testing substrate: RTL+real-backend (primary) + Playwright (edge-coverage) (Epic #1247)

**Canonical testing-substrate** (post-Pegasus shipped state, 2026-05-14):

- **RTL+real-backend** is the PRIMARY e2e substrate for renderer
  behaviour: vitest + jsdom + @testing-library/react + a real bpmux
  backend subprocess spawned by globalSetup. Each test exercises real
  REST + real WebSocket + real renderer round-trip; only Electron's
  main process is absent. Per-test wall-clock is <500ms (per #1303
  AC8); the backend-spawn cost amortizes once per test-run.
- **Playwright** is RESERVED for edge-coverage that RTL cannot
  faithfully replicate: cross-process IPC, native OS features
  (clipboard / file-pick), and visual-class assertions (pixel-diff,
  screenshot regression). Per #1305 audit Bucket-2.

**Wall-clock empirical-win** (per #1304 + #1306 + #1335 deliveries):

- Playwright per-spec: ~1.0-1.5s wall-clock (Electron launch +
  Chromium startup + Playwright bootstrap + teardown)
- RTL per-spec: ~73-167ms wall-clock once globalSetup amortized
  (~10-13× faster per-test)
- Cumulative win compounds across each migrated sub-batch as the
  globalSetup amortization spreads over more tests

**Substrate-foundation**:

- **#1303** RTL+real-backend infrastructure: vitest globalSetup spawns
  bpmux subprocess; bpmuxShim installs window.bpmux pointing at the
  spawned backend; renderApp fixture wires it for tests. Per-run
  serial pool (vitest.config.ts: `pool: 'threads',
  poolOptions.threads.singleThread: true, fileParallelism: false`) so
  parallel workers don't race against the shared backend message-
  store + roster-store.
- **Storage trait `clear_messages()`** (#1306) — test-only message-
  store wipe so reset+seed pattern works deterministically across
  tests sharing the per-run backend. Gated at REST boundary by
  `BPMUX_TEST_SEED_API`; production never registers the route.
- **Test-seed-API routes** (gated by `BPMUX_TEST_SEED_API=1` env-var):
  `/api/test/seed_messages`, `/api/test/seed_pins`,
  `/api/test/seed_reactions` (per #1290 Part A) +
  `/api/test/reset_messages` (#1306) + `/api/test/seed_roster`
  (#1335, dispatches via `OrchestratorAction::Roster` canonical
  chain so `mirror.roster` propagates).
- **Shared test fixtures**: `applications/electron/test/fixtures/`
  exports `renderApp` + `seedMessages` + `resetMessages` +
  `seedRoster`. Each helper one-liner; tests compose them in
  reset→seed→render order.
- **Recipe doc**: `docs/design/ci-migration/playwright-to-rtl-recipe.md`
  documents the migration mechanism + state-isolation pattern +
  sister-shape-variance discipline (split sub-batches when archetype
  doesn't cover; ship substrate-add within slice per Ryan-canon
  no-defer).
- **Parity-log artifact**: per migrated sub-batch, a
  `parity-<commit>.log` captures per-spec parity-evidence + wall-
  clock comparison + falsifier-coverage matrix (sister-canon
  established at #1304 + #1306 + #1335).

**Sub-batch status (per #1305 audit decomposition)**:

| Sub-batch | Family | Tests | Status |
|---|---|---:|---|
| 4.1.A | human-message-* sister-OK | 4 specs / 6 tests | ✓ shipped #1306 (commit ff453dc) |
| 4.1.B | human-message-* divergent | 3 specs / 4 tests | ✓ shipped #1335 (commit 95783f9) |
| 4.2 | real-backend.spec.ts + dev-smoke-* + human-board-* + human-chat-input-* + human-drill-in-* | ~22 specs | pending; blocked-by: none beyond #1303 infrastructure (sister-shape extension of 4.1 pattern) |
| 4.3 | route-mock specs needing `seed_story` | ~25 specs | pending; blocked-by: `seed_story` test-API exists; sister-shape extension |
| 4.4 | iteration-marker route-mock specs | ~6 specs | pending; blocked-by: marker-API gap (per #1305 audit gap-list — markers substrate retired at #1302; spec-class may no longer apply post-retire) |
| 4.5 | message-seed specs (extended seed fields) | ~5 specs | pending; blocked-by: message-seed extension API |
| 4.6 | reaction/pin/iteration-marker specs | ~5 specs | pending; blocked-by: seed-API gaps per #1305 audit |
| Race/sync class | specialized real-backend (Bucket-1.5) | ~3 specs | pending; specialized; defer to final-close-out |

**Resurfacing-condition for final close-out** (per
`feedback_yagni_deferral_names_resurfacing_condition`): when
bulk-batches 4.2-4.6 ship AND Playwright-keep ≤10% of original suite
achieved, file new close-out story scoped to the final transition
(retire shared-dev-host Playwright contention + pre-push hook update
for RTL-primary + ops-doc final-state capture + ratify completion at
Epic #1247 terminal-state). Search phrase for future PM:
`"final close-out resurfacing-trigger"` — grep architecture.md or
recipe.md.

**File-trigger phrase**: file new close-out story when (a) all
bulk-batches 4.2-4.6 ship (sub-batch sister-stories all reach
terminal-state) AND (b) the remaining Playwright suite is verified
≤10% of the original ~120-spec total. The new story's scope:
retire shared-dev-host Playwright contention + finalize hybrid
testing-substrate per the original (now-deferred) #1307 AC chain.

**Stage-fit**: this substrate strengthens v3 internal-bounded-context
boundaries (testing-capability has a canonical RTL+real-backend seam
with Playwright as named edge-case). Pre-Epic, all e2e ran via
Playwright + Electron — shared-dev-host contention saturated the
single self-hosted runner. Post-shipped-state, the canonical RTL
substrate runs ~10× faster per-spec on the same hardware.

**Current shipped wall-clock** (bpmux-mac-m1, 2026-05-14, measured at
#1307 close-out):

- vitest full suite (including infrastructure tests + #1304 + #1306
  + #1335 migrations + sub-batch ATP markers): 228 tests / 32 files
  / 23.34s real time (was 215/22 pre-#1304); per-test ~102ms
  amortized
- Playwright remaining suite: ~115 specs (sub-batches 4.2-4.6 + race/
  sync class + Bucket-2 Playwright-keep all still Playwright)

The final target (<6-8min full-suite wall-clock per original Epic
#1247 AC chain) carries forward to the final-close-out story.
Current shipped state captures the architectural foundation +
substrate-canon; final-close-out captures the full-bulk-migration
terminal state.

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
