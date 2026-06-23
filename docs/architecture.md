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
  layers: `board` (canonical store + adapters + RefreshDriver trait)
  / `bridge` (MCP stdio integration) / `github` (GH API client) /
  `mcp` (MCP server runtime) / `orchestrator` (action dispatch +
  state-delta synthesis) / `session` (persistent storage, lessons,
  identity, briefings, bookmarks, pins, reactions, tokens) /
  `transport` (P2P networking, protocols, REST surface, WebRTC).
  TUI lives in `applications/electron/` (TypeScript renderer +
  Electron main), not in this crate; Epic #1247 retired the
  in-crate `terminal/` module in favor of an Electron-owned UI
  fed through the `transport` REST/WS surfaces.
- Explicit module boundaries via `mod.rs` re-exports. Each feature
  group exposes a curated public API; internals stay private.
- No circular dependencies between feature groups. Dependency flows
  downward: `bridge` depends on `board` + `session` + `transport`;
  `orchestrator` depends on `board`; `board` + `transport` depend
  on `session`; `session` is a leaf. The TypeScript renderer talks
  to bpmux only over the REST + WS protocols exposed by `transport`
  — no in-process coupling.

**Approaching v4 (components-within-app) but not there.** v4 requires
independently-built-and-tested components with explicit declared
dependencies. bpmux modules are logically separate but test-wise
coupled (integration tests span features, spin up the full runtime).

## Current frictions

Ranked by pressure toward next-stage progression:

1. **`bridge/` tool responders duplicated `session/` access logic** —
   no shared query layer. Adding an identity field used to require
   touching `session/identity.rs`, `bridge/tools/get_identity.rs`, and
   bridge helpers; coordination cost grew with change frequency.
   **Status (#1363):** wire-shape leak retired via `bridge::view_model`.
   Identity / token-report / bookmark / velocity responses now flow
   through bridge-owned view types (`IdentityIndexView`,
   `TokenReportView`, `BookmarkView`, `VelocityMetricsView`); session
   schema changes are absorbed at the `From<&session::Foo>` translation
   step instead of cascading to the MCP wire shape. Service-utility
   calls (`session::diag::log_diag`, `session::auth_token::*`) remain
   direct — they're infrastructure, not domain types. The shared
   query-layer goal (DRY across tools) is partially met by the view
   module; full coordination-cost-reduction would require also
   collapsing the per-tool repeat of `load_X → write_X` patterns,
   which is downstream of the view-model substrate.
2. **Integration tests require full runtime** — `tests/integration_main_modules/`
   spin up the full bpmux server with session + board + orchestrator
   + transport wired together; the in-process e2e tests under
   `applications/electron/tests/` launch a real Electron instance + a
   spawned bpmux binary in parallel. No component-level test isolation
   in either tier. A v4 move would extract independently-testable
   units (board operations testable without the orchestrator drain
   loop; bridge tools testable against a stub `RefreshDriver` without
   spinning up the canonical store from disk).
3. **Renderer ↔ bpmux JSON-wire shape is the new boundary** — with
   the TUI extracted to Electron + the Friction #2 view-model layer
   landed, the new architectural seam is the REST/WS contract between
   the TypeScript renderer and the Rust bpmux server. Wire-shape
   drift here (e.g., a session::Foo schema change that the bridge
   view_model absorbs into the wire JSON would not propagate to the
   renderer's TypeScript types) is a coordination cost not yet
   substrate-enforced; types are duplicated in `applications/electron/src/renderer/`.
   v4-equivalent fix: codegen TypeScript types from the bridge view
   models, or contract-test the wire shape at the boundary.
   **Status (WS-only renderer, 2026-06-19, commit 7f14d95c):** the
   renderer's DATA READS moved off REST onto the WebSocket (see
   "Renderer data layer: WS-only" below). The boundary is now the WS
   typed-frame contract; the REST surface persists for CLI / MCP /
   main-process. The TypeScript-type-duplication coordination cost is
   unchanged — still the open v4-fix.

## Where v3 is working

The Continuum argues staying at a stage is often correct.

- **Feature-grouping maps to team mental model** — "I'm working on
  session awareness" translates directly to `src/session/awareness/`.
  No search overhead. Code structure matches domain language (DDD
  bounded contexts).
- **Transport decoupled from business logic** — WebRTC support added
  with zero changes to bridge or session. Transport is resilient to
  layer additions because boundaries were explicit.
- **Test harness mirrors module structure** — `tests/harness/`
  contains `bridge.rs`, `mock_gh.rs`, `peer.rs`, `port.rs`, `pty.rs`,
  `story_helpers.rs`; each sister-fixture maps to a code-side
  feature group it supports. Test intent matches code intent.

## Stage-progression recommendation

Stay at v3 for the next 2-3 stories; begin v4 preparation work in
parallel. The Continuum's guidance: progress when friction is urgent,
not speculatively.

First v4 step landed (#1363): `bridge::view_model` boundary now
translates session types into bridge-owned response types. Testable
with mocked session; decouples bridge from session refactor; closed
Friction #2's wire-shape leak class. Sister substrate-pivots in the
same iteration window: #1353 (bridge/tools_stories.rs split into 9
feature-cohered submodules), #1354 (transport/rest.rs split into 13
feature-cohered submodules), #1360 (orchestrator.rs Mutex sites
refactored to transactional-closure pattern), #1361 (snapshot_status
immutable-derivation).

Candidate next v4 step: extract a `board::view_model` mirror at the
canonical-store↔consumer boundary. The pattern's leverage was high
for bridge — single translation point per response class — and the
same shape applies to `Story` / `BoardEvent` types crossing into
renderer wire shapes. Sequential or parallel to the renderer-side
TypeScript-codegen friction (#3 above).

bpmux likely doesn't need v6+ (microservices) at any foreseeable point
— it's a single coordination substrate, not a service mesh. The
Continuum's v3-v5 range is the load-bearing window for this product.

## Storage layer: Strategy-pattern DI (#1289)

bpmux's persistence is abstracted behind two traits, both at
`applications/bpmux/src/session/storage/mod.rs`:

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

**Session `Storage`** — two impls; contract tests prove
FileStore ↔ SqliteStore semantic-parity (#1799 cut message history
to SQLite; `MemoryStorage` is retired):

- `FileStorage` — disk-backed jsonl + json. Atomic-rename + fsync
  semantics stay inside FileStorage; tests' knowledge of those
  idiosyncrasies is zero. Retained for the legacy-jsonl read the
  `migrate-storage` cutover path needs.
- `SqliteStorage` — SQLite-backed (`session.db`); schema defined by
  **refinery versioned migrations** (`migrations/V{N}__*.sql`,
  embedded + run on connection-open), never inline DDL — forward-only,
  guarded by `migrations_are_forward_only_versioned` (any `U`/down/
  rollback file fails the build).

**`GlobalStore` is unchanged** — `FileGlobalStore` (disk, production) +
`MemoryGlobalStore` (in-memory test double).

The session backend is **injected, not env-read in the data path**:
`StorageBackend::{File,Sqlite}` resolves `BPMUX_STORAGE_BACKEND` exactly
once at the composition root (`StorageBackend::resolve()`);
`Session::new` delegates to `Session::with_backend(…)`, which tests call
with an explicit backend — no global-env mutation, no serializing Mutex,
so the storage tests are independent.

**Production behavior preserved by path-injection builders** —
each FileStorage path is `Option<PathBuf>` with builder
`with_<thing>_path(p)`. Default (None) resolves via the existing
`default_<thing>_path()` helpers (env-overridable, e.g.,
`BPMUX_BOARD_STORE_PATH` + `BPMUX_BOARD_OUTBOX_PATH`; home-default
fallback). Production constructs with None; tests construct with
explicit per-test tempdirs to avoid sharing `~/.bpmux/*` files.

**Contract-test pattern at
`applications/bpmux/src/session/storage_tests/`** — `contract!`
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

### Cassiopeia ratification (2026-05-15, Epic #1189 Wave-C)

After Pegasus shipped the iteration_id-axis membership shape, Ryan
rejected the user-job-fit (2026-05-14 21:51 UTC): iteration-membership
should be **positional-implicit** (the marker's icebox position IS the
commitment signal) rather than **foreign-key-explicit** (`iteration_id`
on each member). Wave-C re-decomposed the iteration-Epic into six
vertical slices that pivot the membership-substrate without losing the
user-job:

| Slice | Issue  | Surface                                                                                            |
|-------|--------|----------------------------------------------------------------------------------------------------|
| A     | #1297  | `IceboxItem::Iteration { number }` variant in `icebox_unified`; `create_iteration` positional insert; renderer flows iteration stories through `StoryRow` (Pegasus `<Iteration>` parent-row retired) |
| B     | #1339  | Iteration-marker drag preserves Iteration variant through the canonical reorder; closed iteration markers lock drag                                                  |
| C     | #1300  | `close_iteration` derives members via above-marker positional walk (bounded by previous marker or top-of-icebox); marker stays at position with closed-state visual treatment; promoted entries evict from `icebox_unified`                                                                          |
| D     | #1301  | One-shot Cassiopeia migration: `auto_heal_icebox_at_path` invokes `migrate_existing_iterations_into_icebox_unified` to drop stale `Iteration` entries + append canonical iteration numbers. Idempotent; sister to `migrate_icebox_unified` at #1258 axis. Audit-log: `docs/audit-logs/2026-05-15-cassiopeia-iteration-migration.md`. |
| E     | #1299  | Retire iteration_id-axis surface — shipped + accepted (positional became the sole membership shape; Carina later re-grounded it on canonical identity, see below)      |
| close | #1302  | Iteration-Epic close-out — this audit (2026-05-25); see the Carina ratification below for the post-pivot final state                                                   |

**Positional-implicit-membership canon**: position in `icebox_unified`
relative to iteration markers IS the membership signal. No explicit
add/remove ceremonies; drag-to-reshape is the user-input. `close_iteration`
walks above-marker, bounded by the previous marker. Closed iteration
markers stay at their position (Slice C's `columnFor` routing + Slice
B's drag-lock).

**Substrate sole-source**: `icebox_unified` is the canonical positional
substrate. The `iteration_id` field remains on the canonical-store
`Story` model during the Wave-C transition window but is no longer the
membership signal — Slice E retires its consumer surface atomically.

**User-job invariant preserved**: per
`feedback_architecture_pivot_preserves_user_job_invariant`, the pivot
re-shapes the substrate without losing the user-visible iteration-as-
batch-of-stories experience. Migration documents which existing
iterations carried forward (5 canonical iterations as of 2026-05-15;
Aquila B disambiguation already encoded in as-filed titles — see
audit-log for the named rule).

### Carina ratification (2026-05-24, Epic #1189 Wave-D)

Cassiopeia made positional-implicit membership the canon but kept
attribution **name-derived** — a leak of the GitHub-issues storage into
the domain. Three user-job failures surfaced on the board 2026-05-24: a
newly-created iteration inherited a retired same-named iteration's
completed stories (name-collision); the NOW view auto-picked a current
iteration (lowest-# fallback) instead of honoring an explicit choice;
and the acceptor-badge guessed from story-TYPE, collapsing
awaiting-owner / dev-accepted stories to a wrong "team" badge. Carina
(Wave-D) pivots attribution from name-derived to **canonical-identity-
derived** while preserving the positional user-job, and corrects the
NOW-derivation + acceptor-render.

**The synthesis — `parent-iteration-#<number>.<position>`**: Carina did
not undo Cassiopeia's positional membership; it grounded it on a stable
identity. The membership label encodes BOTH the iteration's canonical
issue-**number** (identity — survives a rename, immune to same-name
reuse) AND the story's **position** within the iteration (preserving
Cassiopeia's positional-order user-job). `decode_parent_iteration`
(`board/adapters/gh_label_codec.rs`) parses it to `(u64, u32)`. One
label carries both axes; attribution keys on the number, order reads
from the position.

| Slice | Issue | Surface |
|---|---|---|
| 1 | #1553 | Membership attributes by canonical iteration-number (`parent-iteration-#N`), not the legacy `iteration-{theme}.N` name-match — a new iteration can't inherit a retired same-named iteration's members, and a rename can't detach them. Name-match path retired. |
| 2 | #1540 | Explicit current-designation: `BoardEvent::CurrentIterationDesignated/Cleared { number }` domain verbs; `designate_current_iteration` is a deliberate PM-fire (no auto-set on create/open), rejects a closed iteration, board-singleton replace-semantic (designating clears any prior current-holder). Closing clears the designation; NOW reads null-if-none — no lowest-# fallback. |
| 3 | #1549 | NOW work-list IS the designated iteration's members: `Story.is_current` is the single derivation-root feeding the existing collapsible state-sections (Ready → In Progress → Desk Check → Acceptance), scoped to the current iteration — header/name + EPICS + member-list all re-derive off the one designated-id, never the global all-iterations list; honest-empty when none designated. |
| 4 | #1551 | Acceptor-badge reads the honest acceptor-class: `decode_acceptor` maps the `awaiting {team,owner,dev}` label → `Acceptor` enum (returns `None` → honest neutral when absent), never inferred from story-TYPE. The owner can tell at a glance which stories need HIS walk. |

**Close-promote retirement (#1556 — behavior change)**: Cassiopeia's
Slice C promoted above-marker stories to ready on close. Carina
**retires** that (Ryan-directed 2026-05-24): `close_iteration_canonical`
(`transport/rest/iteration_close.rs`) is now a **terminal close** — it
fires `CurrentIterationCleared` then `StoryClosed`, with `promoted: []`
and no member state-transition. Members stay in their state; opening /
designating the next iteration is the deliberate promotion gesture. This
is the one Litetracker reference-behavior consciously superseded rather
than preserved — the membership/designation pivot made auto-promote-on-
close the wrong default (it conflated close with hand-off). The #1258
ready-queue-empty guard still gates close — a **global** invariant, not
iteration-scoped: `collect_ready_queue_blockers` checks `all_stories()`
for `Ready && !is_closed`, so closing any iteration returns 409 while any
story sits in the Ready queue (not just the closing iteration's members).

**Edge-isolation (storage stays in the adapter)**: the domain speaks in
verbs (`CurrentIterationDesignated/Cleared`) and typed fields
(`Story.is_current`, `Story.acceptor`); only the GitHub adapter names
the `current-iteration` / `parent-iteration-#N` / `awaiting-*` labels
(`gh_label_codec.rs` + `is_current_from_labels` / `acceptor_from_labels`).
Per `feedback_storage_serialization_stays_in_the_adapter` the render and
stories read the typed domain field, never the storage label.

**Chokepoint inheritance**: `BoardStore::apply` (`board/store/apply.rs` +
`store/mod.rs`) syncs `is_current` + `acceptor` from labels on every
`StoryCreated` / `LabelsRefreshed` and flips `is_current` on
`CurrentIterationDesignated/Cleared` — the Carina fields inherit the
#1282 fire-ordering invariant automatically, same as the Cassiopeia
substrate.

**User-job invariant preserved**: per
`feedback_architecture_pivot_preserves_user_job_invariant`, the three
positional-experience reference-behaviors survive (positional-order,
drop-to-add implicit membership, drag-to-reshape); the fourth
(promote-on-close) is the deliberate supersession above. The iteration-
as-batch-of-stories experience is unchanged; the attribution underneath
moved from name to identity.

**Strangler-fig sister-pattern**: Pegasus (iteration_id foreign-key) →
Cassiopeia (positional-implicit) → Carina (`parent-iteration-#N.<pos>`
identity+position) is the durable reference-arc for future substrate-
pivots: ship the new shape alongside the legacy path, prove it, then
retire the legacy seam atomically — and when an earlier pivot's
convenience (name-derivation) leaks storage into the domain, the fix is
to ground attribution on a canonical identity, not to add another
parallel path. Audit-trail: #1302 (2026-05-25).

## Renderer data layer: WS-only (2026-06-19, commit 7f14d95c)

The TypeScript renderer's data READS moved from the REST surface to the
WebSocket. Board cold-start (`board/useBoardFetch.ts`) and chat history
(initial hydrate + older-history pagination) now request over the WS —
the renderer no longer calls `GET /api/board` or `GET /api/messages`.

**Prior art**: Ryan's Rails ActionCable channels (REST-like-over-WS —
a `hydrate` action on connect + live deltas on the same stream). bpmux
adopts the request/response half on top of its existing broadcast+mirror:
a `BoardHydrate` / `*Hydrate` / `HistoryPage` `ClientMessage` → a
caller-only reply over the per-connection `reply_tx`. Handlers live in
`transport/ws/hydrate.rs` (`handle_board_hydrate` / `handle_comments_hydrate`
/ `handle_history_hydrate` / `handle_history_page`), each mirroring the
comments-hydrate archetype. ONE stream — bpmux is a single local shared
workspace, NOT multi-tenant, so there is no per-resource stream routing.

**Caller-vs-everyone split**: shared mutations broadcast to all
(existing mirror); hydrate results + action errors reply to the
INITIATING connection only. `useBoardFetch` fires `requestBoardHydrate`
on identify + reconnect and consumes the existing `board_snapshot` delta.

**The REST surface STAYS** — it is the substrate for the bpmux CLI
(gated `/api/rpc/*` write verbs + `/api` reads, off-MCP), the MCP bridge,
and the main process (healthz, electron-capture). The migration removes
REST from the RENDERER, not the server. This reshapes Friction #3: the
renderer↔bpmux boundary is now the WS typed-frame contract, not REST/WS.

**Stage-fit**: v3-strengthening — the renderer↔transport seam is one
typed-frame contract instead of two (REST + WS). Same strangler-fig
discipline as the iteration/storage pivots: each consumer's REST read
retired as its WS hydrate landed, e2e green every slice.

**Test-substrate consequence + a bug it exposed**: every e2e spec that
route-mocked `/api/board` or `/api/messages` had to migrate to WS-driven
fixtures (`launchWithMockGh` + rpc for board state; the `seedMessages`
launch opt for chat history) — the mock was dead the moment the renderer
stopped calling the endpoint. De-mocking surfaced a real producer/
consumer bug: `board/adapters/json.rs` curated labels and DROPPED
`blocked-by:N` + `desk-check-fail`, so the blocker-reference and rework
badges had only ever rendered against hand-authored MOCK data. Fixed by
passing those labels through (`is_renderer_badge_label`). Lesson: the
test-doubles bought isolation at the cost of fidelity; the migration's
value included finding what the mocks hid.

## Role identity: config-driven, not hardcoded (2026-06-19, commit 7f14d95c)

Roles are no longer baked into source. The role SET — ids, count, the
owner — comes from the workflow config (`WorkflowConfig.role`:
`{ id, purposes }`; `workflow/bundled_default.rs` is the overridable
default). Everything downstream derives from it:

- The owner is a workflow role like any other (`owner_role_id()` =
  `role_id_for_purpose("owner")`); its id is the generic `owner`, not a
  personal name. This makes the owner symmetric with the other roles and
  unblocked owner-walk acceptance over RPC — the owner-only `accept_story`
  gate is now drivable as agent `owner`, which the RPC allowlist accepts.
- The RPC agent allowlist derives from `current_workflow().role` ids
  (`transport/rpc_routes/validation.rs`), not a hardcoded
  `["dev","pm","designer","owner"]`.
- The renderer's identity / owner-detection / addressable set, and the
  avatar-color hashing (`roles/roleColor.ts`), are role-agnostic.
- The CI-actions URL derives from `BPMUX_REPO` config, not a hardcoded
  repo handle.

No production-code path enumerates a fixed team. Principle (Ryan,
2026-06-19): "this app is to be usable by anyone, including myself."
**Open follow-ups (deferred — not this refactor)**: dynamic per-role
CSS theming is a designer concern (the static `--role-*` tokens remain);
a renderer that can be ANY role at runtime (multi-user identity) is a
feature. `RosterRole` + the purpose vocabulary stay the stable
classification layer.

**Stage-fit**: v3-strengthening — the role taxonomy has a single config
seam instead of scattered literals across backend, renderer, and plugin.

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

## How this architecture evolves

The substrate improves the same way every pivot above happened: by
continuously **migrating the code to better patterns as they are
learned** — not by holding it still. Refactoring is not a scheduled
project; it is the ongoing act of moving the substrate forward (clean as
you go, pay debt at sight). Each migration lowers the cost of the NEXT
change, because the property that makes change cheap — small
single-purpose files, readable names over comments, behavior-coupled
tests, gates that fire at the moment of the mistake — is the same
property the next better pattern reinforces. The WS-only migration was
cheap precisely because the prior decomposition + behavior-coupled tests
existed: swapping the entire transport under the renderer was a
fixture-swap, not an assertion rewrite, because the tests assert what the
user sees, never how the data arrived. Today's work pays the next change
forward the same way.

**Feedback speed is load-bearing, and the rule is faster-NOT-thinner.**
Feedback speed sets the safe step SIZE: a sub-30s unit signal lets a
small step be taken and corrected in seconds; a 7-minute e2e forces
batching. Small steps mean less can break before detection — fast
feedback → small steps → tight error-correction → small blast radius.
When the slow tier bites (the e2e), the move is to parallelize / reuse
the app+backend / cache / shard — NEVER trim coverage to buy speed.
Cutting tests to go faster destroys the very feedback that makes the
speed worth having.

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
