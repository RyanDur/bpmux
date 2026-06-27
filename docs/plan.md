# bpmux — The Plan

> A living, visible plan so the owner can see direction and help shape it,
> and so the next AI instance knows what we are building and what to do next.
> Draft — owner (Ryan) refines. Read this FIRST.

## What we're building (the vision)

bpmux is a **generic agent-coordination substrate** — messaging, terminals,
presence, sessions, sync. The backend should be a **small, legible Redux
pipeline**: so legible that a fresh instance can read the folder names and
know what each part does and what to do next. The **AI workflow** (how this
team happens to run — stories, gates, iterations, acceptance) is NOT part of
the substrate; it lives in a light layer above (config / skills / hooks).
The frontend and backend share **one typed wire contract**, generated from a
single source of truth, never hand-duplicated.

## The four goals (owner's words)

1. **Stable front↔back contract.** One typed source of truth for the wire;
   generate the TypeScript types from the Rust structs, never maintain both.
2. **AI-process out of the app.** The substrate stays generic; the PM/dev/
   designer process moves to a light layer, out of the core.
3. **Simple, self-informing backend.** The structure prompts the next
   instance — read the folders, know what each does and what's next.
4. **Redux made visible.** The code uses Redux vocabulary — store, slice,
   action, reducer, middleware, selector, dispatch — so the architecture
   teaches itself to the owner and the next agent.

## The architecture — agnostic store + composed slices (validated in `store_spike/`)

The central store is **generic infrastructure**, agnostic of every domain.
Three layers, each knowing only what it must. This is proven end-to-end in
`applications/bpmux/src/store_spike/` — runnable, fully typed, no `unsafe`, no
`Box<dyn Any>`; it is the reference for the real thing.

**1. Kernel — the generic store (`createStore`).** `Store<S, A>`: a state
container, a composed **middleware** chain, a root **reducer**, and
**subscriptions**. Imports zero domains. `dispatch` walks the middleware chain
and then the reducer; subscribers fire only when the state actually changed.

**2. Slice — the domain (`createSlice`).** Each domain owns, in its own module,
its `State`, its `Action` enum, its pure `reduce(&mut State, &Action) -> Changed`,
its selectors, and its `Middleware`. It references nothing but the kernel's
`Changed` / `Middleware` — not the store, not a sibling, not the app action.

**3. Composition root — the only place that names domains** (`combineReducers`
+ `applyMiddleware` + `configureStore`). It composes:
- state: `struct AppState { roster: RosterState, ci: CiState, … }`
- actions: `enum AppAction { Roster(RosterAction), … }` + `From<RosterAction>`
  (so a domain never names the others)
- reducer: `app_reduce` matches `AppAction` → the slice's `reduce` over its
  sub-state — this IS `combineReducers`
- middleware: the composed chain

Adding a slice = one field, one variant + `From`, one match arm, and (optionally)
its middleware in the chain — ALL here. The store and every other slice are
untouched.

**Middleware** is Redux's `api => next => action`, flattened to one method:
- `api.dispatch(a)` — a NEW action from the **TOP** of the chain (re-enters
  every middleware; how effects/async emit follow-ups).
- `next(a)` — carry THIS action to the **next** link, toward the reducer.
- `api.get_state()` — read current state.

The reducer is the chain's **terminal** link, so every middleware runs **before**
the reducer by construction. Middleware run lock-free; only the reducer takes
the state lock.

**Why static composition, not a runtime registry.** "Register with the store"
lands, in Rust, as static composition at the root: Rust's enums are closed, so
the one place that lists the domains is `AppAction`, and the composition root is
exactly where that belongs. A runtime registry (adding a slice after the store
is built) would force `Box<dyn Any>` erasure — and bpmux's slices are all known
at compile time, so we don't need it.

**How bpmux maps onto this.** The god-object `src/store/mod.rs` is what the
kernel replaces. The board's `project_label_delta` (codec BEFORE the pure
reducer, enforced by the `board_reducer_does_no_label_decode` fitness function)
is **already a middleware** — it just needs the formal slot.

This also resolves the substrate/process tension below: the agnostic store is
pure substrate; every domain owns its slice; an AI-process domain (the board)
takes its slice WITH IT to the light layer — the store doesn't care where it lives.

### Per-domain kernel stores — slices need NOT share one AppState (owner, 2026-06-26)

The kernel is generic; "compose at the composition root" does not require a
SINGLE `AppState`. A small slice family (roster/ci/reactions/sync) shares one
`AppState` because they're co-located substrate. A **large bounded domain keeps
its OWN `kernel::Store`**: the board now runs on a self-contained
`Store<BoardState, BoardAction>` inside `product_management/board/` (state +
action + pure reducer + the label codec as middleware + selectors), NOT folded
into the app `AppState`. This keeps module boundaries clean — the composition
root stays small and doesn't import the big board domain. So "board becomes a
selector over the story slice" means a selector over the board's OWN store, and
the earlier "fold every slice into one AppState" framing is superseded for large
domains. Revisit unification only if a real cross-slice need appears. Landed
per-domain at `a76fc36c` (`BoardStore` reimplemented on the kernel, public API
preserved).

## Where we are now (honest)

- ❌ **The `Store` is a god-object, not agnostic.** `src/store/mod.rs` hardcodes
  `ci_store`, `reactions_store`, `roster_store` as named fields, with per-domain
  methods (`reactions_store()`, `add_ci_listener()`, …) and per-domain listener
  vecs, and imports `CiStore`/`ReactionsStore`/`RosterStore` directly. The slices
  live as submodules INSIDE `store/`. There is NO registry — every slice is baked
  in by name. This is the core thing to fix: make the store agnostic + a registry,
  and move each slice into its own domain.
- ✅ It does have the right *vocabulary* at the top (`action.rs`, `dispatch.rs`,
  `snapshot.rs`) — but the composition is monolithic, not registered.
- ⚠️ **The board is the exception and the tangle** (`product_management/board/`):
  - It has its own redux-*shaped* pipeline — `board/store/apply_events/`
    (reducers), `board/store/middleware.rs`, `board/orchestrator/` (sync) —
    but it is NOT a slice in the top-level `store/`.
  - It is named in **domain terms, not Redux terms**: `apply_events` (not
    `reducers`), `orchestrator` (not `middleware` / a sync listener),
    `BoardStore` (not a slice). So it does not yet TEACH redux.
  - **GitHub adapters live inside the board** (`board/adapters/github/`)
    instead of at the edge — domain logic tangled with an external-service
    adapter.
  - The board's PROCESS (the In-Progress/Desk-Check states + gates) is
    hardcoded, not config — that's what goal #2 lifts to a per-instance,
    shareable config layer (see the workflow-engine section above). The
    board ENGINE itself stays as substrate.
- ✅ **Recent groundwork** (done, CI-green): the *board fold* made every
  derived field a **selector over `raw_labels`** (`select(state)`); the
  *`view_model::StoryView`* made the board-card wire a **typed struct**
  (this is what makes goal #1 possible). Both were right and not wasted —
  they cleaned up the engine that stays; they're just named in domain terms,
  so they don't yet read as Redux.

## The board is a configurable workflow engine, NOT process-to-delete (owner, 2026-06-25)

The board is a **workflow state machine** whose job is to keep work flowing:
- a fixed **entry** — Ready,
- a **configurable process** in the middle — today In Progress + Desk Check
  (this team's current process),
- a fixed **exit** — Acceptance.

The **engine** (entry → configurable middle → exit, each middle state optionally
gated) is generic **substrate** — it STAYS, as a clean slice. What is "AI-process
out of the app" (goal #2) is the **process DEFINITION**: the middle states and
their gates become **data/config**, not hardcoded transitions. Future: each
**instance defines its own process**; process definitions are **shareable**, but
an instance picks/defines its own. The current In-Progress/Desk-Check/etc. become
the *default* process config.

So the board fold + `view_model` were not wasted — they cleaned up the engine
that stays.

## The roadmap domain model (the default — structure is fixed, the labels are config)

The planning structure the board executes over. Definitions are the owner's
default; another instance may rename every term (see the principle below).

- **Outcome** — the *why*: a measurable change in behavior that drives results.
  Priority-ordered.
- **Iteration** — a priority-ordered, mutually-**decoupled** set of stories that
  realizes an outcome incrementally.
- **Story** — the unit of work, priority-ordered within its iteration, part of
  one **epic**. (Vertical-slicing is a team preference, i.e. config.)
- **Epic** (owner's definition, kept over Labs "Output") — a **capability** that
  serves an outcome and **may span multiple outcomes**. Cross-cutting.

Two orthogonal groupings over the same story: **delivery** (Outcome → Iteration →
Story) and **capability** (Epic → its stories, across iterations/outcomes). A
graph, not a tree.

- **Priority is ordering** — backlog position is the truth (top = most important).
  **Velocity** (avg points / iteration) turns ordering into a time *forecast*.
- **Now / Next / Later (horizon) is a VIEW**, not a stored attribute — a
  projection of (ordering + velocity) onto a temporal presentation. The backend
  holds ordering + velocity; the roadmap view derives the bands.
- **Board vs IPM:** the board is the *execution* state machine over these stories
  (entry → configurable process → exit); the **IPM** is the separate *planning*
  flow (groom → estimate → promote-to-backlog). Different state-sets.

## Persistence posture is per-slice (and conversations are the real durable artifact)

This whole arc began with adding a DB to durably store the **chat conversations**
— that is the real durable artifact, and the schema for it (message identity, the
conversations entity, total/normalized) is the origin. There is no single
"save the state" rule; **each slice owns its persistence posture** as its own
port + adapter, and the kernel is agnostic of all of it:

- **conversations / messages → DB-durable.** The real artifact; what the DB is FOR.
  Concern: the total/normalized schema with **message identity from birth**
  (`sender` NOT NULL, conversation entity). The message-id slice already landed.
- **stories (the entity) → persisted behind an ADAPTER (GitHub issues).** The STORY
  is what's saved as a GitHub issue — not "the board" (the board is a VIEW + workflow
  OVER stories, no persistence of its own). The persistence lives **behind the
  `SourceAdapter` port**: the story slice (Redux, GitHub-agnostic) has `drain`/`ingest`
  effects over the port; the GitHub impl (issue I/O + codec) is the adapter at the
  edge. In-memory projection hydrated from GitHub + a *temporary* outbox; never in our
  DB. Generic — any entity backed by an issue tracker is a new adapter, zero slice
  change (stories today, outcomes maybe next). NOT a "store" (that's Redux's word).
- **ci → ephemeral.** Watched external state; not persisted.

Cross-cutting principle both ends share: **total identity from birth** — every
entity gets a stable local identity the moment it exists; external ids (a GitHub
issue number) are *synced attributes*, never the identity. Messages have it;
stories need it (the id-on-create target above).

### Principle: structure is the engine; names, process, and rules are config

The roles and relationships (outcome/iteration/epic/story; entry → process →
exit) are the **generic engine** that stays in the app. The **specific words and
rules** — the entity NAMES, the process states + gates, the slicing preference,
the horizon bucketing — are **per-instance, shareable config**. This is goal #2
stated precisely: bpmux is a generic engine; everything team-specific is
configuration; the owner's definitions are the default. So the backend models
structure with **stable internal identity**, and a config/display layer carries
the renameable labels.

## The conversation / message schema (the durable core — where this began)

The DB exists to durably store the **chat conversations** — the real durable
artifact. The schema is **total / normalized**, with **identity from birth**.
(This plan was previously saved only in agent memory, invisible to the owner —
hence this section; durable plans live HERE.)

**The conversation model.** A **conversation = a static recipient combination**,
built at user registration over the roster. `bpmux` is a **uniform user** (no
`from == "bpmux"` special-case). The **owner is a participant of every
conversation** — that's why the owner sees/interacts with all, not a god-view
overlay. Scale: all (everyone) → combinations → 1-on-1 (`{owner}+1`). Each message
carries **three total identities**: its own `id`, `sender` (author), and
`conversation_id` (audience). The legacy `to` / `addressee` / `explicit_privacy` /
`BROADCAST_ALL` / `reactions.channel` are **denormalized shadows** of the
conversation — they collapse into `conversation_id` + membership
(`explicit_privacy` is derivable: scoped ⟺ conversation ≠ all). P2P: WebRTC passes
**messages only** (no conversation/roster sync), so `conversation_id` is a **local
uuid**; audience travels on the wire as participant identities, mapped to the
local conversation at ingest.

**Target schema (V1):**
- `users(id TEXT PK uuid, session_id UNIQUE, name, role, kind CHECK(agent|owner|system), last_delivered_at INTEGER NOT NULL DEFAULT 0, …)`
- `conversations(id TEXT PK uuid)`
- `conversation_participants(conversation_id FK, user_id FK, PK(both))`
- `messages(seq PK, id TEXT NOT NULL UNIQUE, sender TEXT NOT NULL → users, conversation_id TEXT NOT NULL → conversations, created_at, message_type, payload)`
- `reactions(message_id FK, user_id FK, emoji, PK(message_id,user_id,emoji))`
- `outbox` / `bookmarks` / `surfaced_topics` unchanged; `meta` until the jsonl→sqlite cutover.

**Slice sequence (S1 landed; S2–S8 remain):**
- ✅ **S1** — `messages.id` minted from birth (`ensure_id`, NOT NULL UNIQUE). Landed (`13adba8e`, CI-green).
- **S2** — id on the wire + renderer keys by `message.id` (retire `${ts}-${seq}`).
- **S3** — `sender` totality: NOT NULL; set `sender_id` at EVERY producer; `resolve_sender` total.
- **S4** — cursor totality: `last_delivered_at` NOT NULL.
- **S5** — conversations foundation: the two tables; build static combos at registration (owner in every one).
- **S6** — `messages.conversation_id` NOT NULL FK; resolve audience → local conversation at append.
- **S7** — flip delivery + retire shadows: `should_deliver` reads conversation membership; remove to/addressee/explicit_privacy/BROADCAST_ALL from struct, wire, renderer.
- **S8** — reactions normalize: the join table; key reactions by message id + conversation.

**Persistence-as-projection (the CQRS framing that unifies this with everything
else).** The action/event **log is source of truth** (jsonl today); the store
**materializes** it; SQLite + GitHub + WS-broadcast are **eventually-consistent
projections** driven by non-blocking **listeners**. So "save the conversation to
the DB" is a DB-projection listener on the messages slice — not a synchronous
write in the action path. Same shape as the board's GitHub sync, the WS broadcast,
and CI's poll: one kernel, slices own their state, projections fan out via listeners.

## The substrate / process line (the rest)

- **Substrate** (stays, each a slice): messaging, rooms/terminals, presence/
  awareness, sessions, routing, storage, reactions, roster, **the board engine**
  (the workflow state machine above), and **`ci`** — a generic *watch an external
  resource's state* primitive (running/passed/failed by polling), with the
  resource (*GitHub Actions* today) as a pluggable edge adapter. `ci` is the clean
  worked example of the model: an agnostic slice + a poll **effect** (timer →
  fetch via the port → `api.dispatch(StatusChanged)`) + an edge adapter.
- **The STORY entity's persistence — named by prior art (NOT "sync", NOT "the
  board", NOT a "store").** Two grounded concerns, not one:
  - **The I/O edge = the existing `integration-gateway` triad** (Cockburn Ports &
    Adapters + Fowler PoEAA Gateway + dam5s, enforced by `boundary_tests`):
    **Port** `SourceAdapter` (domain) → **Adapter** `GitHubSourceAdapter` (edge) =
    **Gateway** `OctocrabStoryTracker` (octocrab→`Value`) + **Mapper** (`Value⇄Story`
    codec, the ACL). New issue tracker = new adapter, zero slice change.
    (Nit: the port being NAMED `SourceAdapter` blurs port-vs-adapter — it's really a
    **Repository**; rename when touched.)
  - **The near-side replication that DRIVES the adapter (Redux, provider-agnostic).**
    This is canonical *eventual consistency in Ports & Adapters* — handled at the
    boundary, never in the core (pure reducer):
    - `drain` + `outbox` = an **output/driven adapter** + the **Transactional Outbox**
      pattern (durable buffer → push through the port → confirm/rollback). We go finer
      than "the adapter handles retries": durability + retry live near-side in the
      outbox/relay; the adapter is just the publish call.
    - `ingest` = the **input side** — a **CQRS read-model projection** (poll the port
      → dispatch into the local view, which is NOT the source of truth). We poll;
      a webhook would be a true input/primary adapter, same shape.
    - `reconcile` = conflict resolution (merge inbound with the optimistic window —
      merge-not-clobber).
    This logic is the story slice's effects, NOT "the adapter" — it uses the adapter
    through the port.
  - **id-on-create = "read-your-own-write," the textbook EC-in-hexagonal case.** The
    canonical solution IS our design: never query the eventually-consistent store from
    the core; the **primary (driving) adapter — the WS — notifies the client when the
    read-model caught up.** drain → GitHub assigns the id → follow-up action → reducer
    fills it → WS re-emits → the client sees the id. (Same boundary-not-core move as
    horizon-is-a-view.)
  - **Stories are NOT persisted to our DB.** GitHub IS the durable store / source
    of truth; the in-memory store is a **hydrated projection** (the fold made
    `raw_labels` the faithful GitHub representation). The ONLY local persistence is
    the **outbox**, and it is **temporary** — pending mutations sit there crash-durably
    until GitHub confirms, then they're removed. The DB is for other substrate
    (messages), not stories. ⚠️ VERIFY: the current code isn't saving stories to the
    DB (`orchestrator.storage`) against this intent — possible drift to fix.
  - NOTE: today's `BoardStore` is really the **story store**, misnamed for its view —
    rename it (board becomes a selector over the story slice).
  - **What `sync` IS today (verified from code):** an optimistic-with-rollback
    eventual-consistency engine. `store/sync/` (`SyncAction`): `StartIngestPolling`
    (poll IN), `StartHydrationPolling` (poll lazy bodies), `StartUpdateGithub{mutation_id}`
    (drain OUT), `MutationConfirmed` / `MutationRolledBack{server_story}`.
    `orchestrator/`: a durable on-disk **outbox**, **drain**, **ingest**,
    **coalesce** (merge queued mutations), **conflict** (classify+resolve),
    conditional **fetch** (etag), **status** (pending/stale/dropped), and a
    **race_window** barrier. The `conflict` + `race_window` machinery are scars
    from real bugs — load-bearing; a refactor must PRESERVE this behavior.
  - **The owner's intended SHAPE** matches it (local-first optimistic → async drain
    → poll-in to ingest), all expressible as board-slice effects + subscribers.
  - **⭐ TARGET (a behavior we do NOT have yet — the problem to solve): optimistic
    create shows its id after the GitHub save.** Blocked by IDENTITY: stories are
    keyed by their GitHub issue number (`Story.number: u64`; store is
    `HashMap<u64, Story>`), so a story can't exist before GitHub assigns it one —
    nothing to render optimistically.

### The story identity model (owner + dev, 2026-06-26 — SETTLED)

A story is a **discriminated union** whose variant *is* its lifecycle, so the two
illegal states (saved-without-a-number, unsaved-with-a-number) are unrepresentable:

```rust
enum StoryKey { Pending(Uuid), Saved(u64) }          // store key / stable handle
struct StoryContent { title, body, state, filed_at, filed_by,
                      free_comments, gate_comments, raw_labels }
struct UnsavedStory { id: Uuid, content: StoryContent }
struct SavedStory   { id: u64, body_etag: Option<String>, content: StoryContent }
enum Story { Unsaved(UnsavedStory), Saved(SavedStory) }
HashMap<StoryKey, Arc<Story>>                          // Arc = cheap reduce-clones, NOT identity
```

- **One identity, no separate `number`+`id`.** Every story has `.id`; the variant
  says what kind — a local `Uuid` while pending, the **GitHub number `u64` once
  saved** (the number IS the id). No `number` field anywhere.
- **The key is the stable handle** — no `ptr_eq`/value-matching. Create mints a
  `Uuid` (in the action, so the reducer stays pure) → reducer inserts `Unsaved`
  under `Pending(uuid)`. The frontend reads `Pending` as "unsaved."
- **Unsaved stories ARE mutable.** Edits target the key (LWW — natural reducer
  order); the key never changes on edit. An edit to an unsaved story becomes its
  **own queued mutation** that drains to GitHub only after the create assigns the
  number — exactly what the existing `race_window` + outbox handle.
- **Save = a type transition, by key.** The GitHub-save **listener** (on the
  board's own per-domain store) creates the issue, then dispatches
  `StorySaved { pending: Uuid, id: u64 }`. The reducer looks up `Pending(uuid)`,
  takes the story shape *currently* there (so in-flight edits aren't clobbered),
  transitions `Unsaved → Saved(id)`, and re-files it under `Saved(id)`; the
  `Pending` key disappears. Then re-broadcast → frontend shows `#id`.
- **The full create flow (owner's words):** dispatch `StoryCreated` (full payload)
  → middleware → reducer adds it immediately → WS-broadcast listener emits the
  *unsaved* story → GitHub-save listener pushes (retry; durable **DB outbox**,
  entry deleted on confirm) → GitHub number → `StorySaved` re-dispatched at the top
  → reducer fills/re-files → re-broadcast. Effects-as-listeners end to end.
- **Create-then-immediate-update, on the sync side (owner):** the **store is never
  blocked** — edits apply to local state immediately, last-write-wins. The **listener
  serializes create-before-update**: it does the initial create and **blocks the
  update-side push until the create returns the id** (you can't PATCH an issue with no
  number yet). While waiting, **incoming updates coalesce** — a newer update replaces
  the pending, not-yet-sent one (LWW), so only the latest is held. Once the id arrives,
  the coalesced update commences and the **normal update → outbox → drain** flow takes
  over. This maps onto existing machinery: `issue_lock` / `race_window_barrier`
  (block update until create responds), `coalesce` (replace pending update while
  waiting), outbox (the normal flow). Applies PER TYPE.

**Two field decisions (owner):** `acceptance_test_path` is **removed** from the
substrate — it's *process*, lives in the body (the eager `parse_acceptance_test_path`
leaves the substrate; the light layer parses if needed). `is_closed` is **collapsed
into `state`** — GitHub open/closed becomes an *adapter mapping* (ingest: closed
issue → terminal state; drain: terminal state → close issue), so the domain never
reads a raw closed-flag (this FLIPS today's `empty+closed → Accepted` derivation:
the adapter now *produces* the terminal state; the `state` enum must name every
terminal/closed state — Accepted and however retired is modeled).

This supersedes the earlier "local id + GitHub number as a synced attribute"
(dual-field) wording. Same "total identity from birth" spirit as the message-id
work, but the number *becomes* the id rather than sitting beside a local one.

**Slice plan (CI-green each; `Story` is pervasive so the type reshape goes via
make-the-change-easy-first):** (1) drop `acceptance_test_path` from the substrate;
(2) collapse `is_closed` into `state` (adapter maps open/closed); (3) extract
`StoryContent`; (4) introduce `StoryKey`, key the store by it (all `Saved` for now);
(5) introduce the `Story` enum + `Unsaved`/`Saved`, migrate consumers behind
accessors (still all `Saved`); (6) the optimistic-create flow — create mints
`Pending`, renders unsaved, save-listener transitions to `Saved`; (7) renderer
(the deferred frontend touch — its own modeling).

#### Implementation decomposition + blast-radius (dev, 2026-06-26)

Firsthand audit of the LIVE tree before reshaping `Story` (it is pervasive —
[[feedback_existing_code_is_not_the_spec]], [[feedback_whole_substrate_consumer_grep_not_component_scoped]]):
- **No `story_type` field** — shape is DERIVED at read via
  `story_type_from_labels_or_title(raw_labels, title)` (~40 sites). So the
  per-shape split is adapter-routing + per-shape reducers, NOT removing a field.
- **Two maps keyed by `.number`** (`HashMap<u64, Story>`): the board store
  (`board/store/state.rs`) and the in-memory adapter (`board/adapters/in_memory/`).
  ~150 `.number` reads, ~96 test construction sites, ~3 prod construction sites
  (reducer `StoryCreated`, github `story_decode`, in-memory seed).
- **For a STORY, open/closed = terminal lifecycle state (owner, 2026-06-26).**
  A story's issue is closed exactly when the story is **Accepted** or **Retired**;
  open = in flight (Icebox→Acceptance). So `closed ⟺ state ∈ {Accepted, Retired}`
  — DERIVED from `state`, NOT a stored axis. The #1144 appearance of a second
  axis (`get_story` showing `is_closed` apart from `workflow_state`; the
  Accepted-but-open "2026-05-02 false-success class") is the **sync window** —
  accepted locally, the GitHub issue-close still pending in the outbox — i.e.
  sync STATUS at the edge, not a property of the story. (My earlier "deliberate
  second axis" read was WRONG; the divergence is the sync window.) So `is_closed`
  LEAVES the domain: the adapter owns the GitHub open/closed mapping (ingest a
  closed issue → terminal state — Retired if the retired label is present else
  Accepted; drain a terminal state → close the issue). `body_etag` leaves too —
  an HTTP conditional-fetch detail that becomes the adapter's own etag cache
  (keyed by issue number); `fetch_body_conditional` drops its etag param,
  `BodyFetched` drops its etag field, the store stops couriering it. Net:
  `SavedStory` carries only `{id, content}`. (Epic/iteration/outcome get their
  own done-ness in their own types — that's why one shared `is_closed` strained
  and felt like a separate axis.)

Turning `Story` into the enum naively breaks `.field` at ~400 sites in one commit
— violates bounded-slice. So **accessor-seam-first** (Beck make-the-change-easy),
each slice CI-green + path-limited:
- **P1 — storage-key seam:** introduce `StoryKey`; migrate both maps to
  `HashMap<StoryKey, Story>`; public APIs stay number-based (wrap `number → Saved`).
  Story struct unchanged. Bounded to board/store + in_memory.
- **P2 — read-accessor seam (by layer):** add `Story` field accessors; migrate
  consumer reads layer-by-layer (bridge / transport / product_management /
  roadmap / iteration), one CI-green slice each. **Identity accessor is
  `id() -> StoryKey`, NOT `number()`** (owner, 2026-06-26): the number BECOMES
  the id (no `number` field at P4), so the domain speaks `StoryKey`. The raw
  `u64` GitHub issue number is an **edge/adapter concern** — only the GitHub
  adapter, the agent-interface/bridge (user types "42"), and the wire/renderer
  (`view_model`) unwrap `StoryKey::Saved(n)` at their boundary. `StoryKey`
  carries its own `Display` ("#42" / draft marker) so display/sort sites use the
  id directly and Pending is handled at the type, not at ~150 call sites.
  (Number isn't ordering — backlog/label position is; the `.number` sorts are
  incidental tiebreakers.)
- **P2b — `is_closed` leaves the domain:** derive open/closed from `state`
  (terminal = Accepted/Retired); the adapter owns the GitHub open/closed mapping;
  migrate the ~70 consumers (`column_for`, roadmap/iteration filters, acceptance
  + reopen logic, `get_story` output — rewrite the #1144 `get_story` tests to the
  single-lifecycle model). Behavior-touching → TDD per consumer.
- **P2c — `body_etag` leaves the domain:** adapter etag cache (number→etag);
  `fetch_body_conditional` drops its etag param; `BodyFetched` drops its etag
  field; the store stops couriering it.
- **P3 — construction seam:** `Story::saved(...)`/builder; migrate the ~96
  construction sites (may bundle per-layer with P2).
- **P4 — the enum:** `Story` → `enum { Unsaved(UnsavedStory), Saved(SavedStory) }`
  + `StoryContent`; accessors/constructors absorb it; consumers untouched; map
  already speaks `StoryKey`.
- **P5 — id-on-create flow:** `CreateStory` mints `Pending(Uuid)`; events carry
  `StoryKey`; save-listener dispatches `StorySaved{pending,id}` → `Unsaved→Saved`.
  Pending-window keying per open-risk #1.
- **P6 — renderer touch** (the deferred frontend; ships with the capability).

P4's enum is the SETTLED design above; P1–P3 only bound the churn to reach it.
Siblings (Iteration/Outcome/Epic) then replicate on the same seam.

### Roadmap shapes are distinct domain TYPES, each its own slice (owner, 2026-06-26)

`Story`, `Iteration`, `Outcome` (and `Epic`) are **NOT** one `Story` + `StoryType`
enum — they are **distinct domain types, each with its own reducer/slice and its
own lifecycle**. To GitHub they're identical issues; the **adapter parses an issue
into the right shape** (by labels/title) and routes it to that shape's reducer (the
stories reducer handles stories, the iterations reducer handles iterations, the
outcomes reducer handles outcomes, …). This is the roadmap domain model above made
concrete, and it's the proper home for the lifecycle question `is_closed` exposed:
each type owns its own done-ness (a story's Accepted/Retired vs an iteration's
*complete* vs an outcome's *achieved*), so there's no single `state` enum to
overload.

Consequences: (a) the **`is_closed` collapse is SUBSUMED** by this split — dropped
as a standalone step; "done-ness" is just per-type lifecycle. (b) the **id-on-create
identity model (`Pending → Saved`) applies PER TYPE** — each shape can be
optimistically created; that design is not wasted, it lands on the *Story* slice
with iteration/outcome/epic as siblings using the same pattern. This is a
**substantial restructure** (today's single `Story`/`StoryType`/`apply_event`/one
store → per-shape types + reducers + adapter dispatch) and gets its own modeling
pass.

**Structure + sequencing (owner, 2026-06-26):** each shape is its OWN slice — its
own reducer, its own actions (`CreateStory`/`UpdateStory`, `CreateIteration`/
`UpdateIteration`, `CreateOutcome`, `CreateEpic`, …), and its own save **listener**
— and those listeners **share the `SourceAdapter`** (GitHub issue CRUD is common;
shape-specific labels/body are the listener's concern, parsing is the codec's;
specialize the adapter only where a shape needs different I/O). There is **no forced
generic identity abstraction** — the per-slice structure IS the design (the apparent
"type explosion" is just legible per-shape slices). This sets the sequencing:
**Story-first, then siblings** — build the Story slice end-to-end (reducer +
`CreateStory`/`UpdateStory` actions + save listener on the board's own store, on the
shared adapter), then add Iteration/Outcome/Epic as sibling slices the same way (no
big-bang split). id-on-create ships on the Story slice; the others follow
incrementally. The per-shape save listeners relocate `store/sync/` into the board
domain (answers the per-domain loose end).

### Store structure — ONE store per domain (owner, 2026-06-27)

RESOLVES the open store-structure crux: **a single reducer, middleware chain, and
listener-set PER DOMAIN** — NOT per-type stores. The board/planning domain stays
the ONE `kernel::Store<BoardState, BoardAction>` it became in P1/slice-3a. The
shapes are **distinct types in that domain's state** (`Story`/`Iteration`/
`Outcome`/`Epic` — separate structs, own decoded fields, held as per-shape typed
collections in `BoardState`). This reconciles the earlier "each shape = its own
reducer/listener" wording: that means per-shape **arms within the single
`reduce()`** (`CreateStory`→story logic, `CreateIteration`→iteration logic, like
today's `apply_event` routing), the single **middleware = the codec** (reads the
type label, decodes into the right shape), and the single **listener-set** handling
save/sync for all shapes (sharing the `SourceAdapter`, routing by type). Simpler
than per-type stores — nothing new to compose. Build **Story-first**: make `Story`
the dedicated story type, then peel `Iteration`/`Outcome`/`Epic` into their own
types within the same store.

**The remaining hard part = the incremental transition (behavior-critical).**
Today ONE `Story` struct backs every shape (discriminated by
`story_type_from_labels_or_title`), and consumers say `Story` everywhere; the
board/roadmap must stay correct through every slice. Likely shape of the
transition (to design carefully, TDD, gates as the safety net):
1. **Codec-decode first:** build the single codec-middleware that decodes a
   GitHub issue's labels → DECODED fields (move the ~13 `*_from_labels` selectors
   INTO the decode; `state` from labels incl. closed→terminal; drop `raw_labels`/
   `is_closed`/`body_etag` from the domain). This cleans the *Story* shape and is
   where raw_labels/is_closed/body_etag actually leave — it unblocks P2-consumer-
   migration + P4.
2. **Peel sibling types:** introduce `Iteration`/`Outcome`/`Epic` structs, route
   ingest by type label to each shape's decode, store in per-shape collections in
   `BoardState`, add per-shape reducer arms, and re-point iteration/outcome/epic
   consumers (roadmap, iteration modules, those bridge tools) to the new types.
3. **Then** the identity/enum/id-on-create (P4/P5) land per shape, Story-first.
Open: exact per-shape field sets; whether a shared identity/lifecycle trait helps.
This is the next modeling step — pace it; it's the load-bearing board logic.

### Labels leave the domain — per-shape codec (owner, 2026-06-26)

`Story.raw_labels` (verbatim GitHub labels, with ~13 `*_from_labels` selectors
decoding it — the board fold) is a GitHub ENCODING leaking into the domain. It
leaves: each per-type **codec** (the anti-corruption Mapper, per shape) decodes
the labels its shape cares about into that type's domain fields on ingest, and
encodes fields → labels on drain. The domain holds DECODED facts, not label
strings.

Labels are largely **shape-specific**, with only minor overlap (owner):
- **Story:** `state` (icebox..retired), acceptor / `awaiting owner`, `blocked`,
  ready-order, `parent-epic-#N` (its epic), `iteration-#N` (its membership).
- **Iteration:** `parent-outcome-#N.P` (membership + position),
  `iteration-review-closed.role`, `iteration-dev-ok.role`, `current-iteration`,
  iteration-order.
- **Outcome:** `horizon.now/next/later`, horizon-order.
- **Epic:** essentially just the type label (capability grouping; children point
  to it via `parent-epic`).
- **Overlap is minor:** the **type label** (feature/bug/chore/epic/iteration/
  outcome) is the only truly shared one — and it's the ROUTING key (the adapter
  reads it to pick the shape's codec). Cross-ref labels (`parent-epic` on a story,
  `parent-outcome` on an iteration) live on the CHILD, owned by the child's codec.

`state` STAYS a domain field (already decoded-and-stored at ingest, NOT a
raw_labels selector); the raw_labels SELECTORS (type, parent_epic, membership,
horizon, acceptor, tags, …) LEAVE — they become each type's decoded fields via
its codec. This is stream-D (GitHub to the edge) + the per-type split (#14): a
substantial restructure, NOT a P2 accessor slice. `raw_labels` migration is
FROZEN until then (don't cement a `raw_labels()` accessor across consumers).

**Open risks to design (not blockers):**
- **Pending-window keying.** `issue_lock` / `coalesce` key on the GitHub `u64`,
  which an unsaved story doesn't have yet. Each shape's create listener must
  serialize create-before-update and coalesce on the **`Pending(Uuid)`** during the
  unsaved window, then hand to the number-keyed path once the id lands.
- **Capability is renderer-gated.** id-on-create's user-visible payoff (the
  optimistic story rendering before GitHub answers) needs a renderer touch; putting
  it behind the whole frontend pass means no demoable value until then — the thin
  renderer touch for *this* feature may ship with it instead.
- Stale note: the `⚠️ VERIFY orchestrator.storage` line above is resolved —
  `storage` IS the intended DB outbox, not story-persistence drift. And `Epic` may
  be a cross-cutting *capability* grouping, not an executable create→save shape —
  confirm when modeling shapes.
  - **Care-point (the "state-delta echo clobber" scar):** ingest/reconcile must
    MERGE, never blind-overwrite — a poll echoing GitHub must not stomp a newer
    local optimistic edit.
- **Process / agent-specific — NOT bpmux's responsibility** (lifts out, to config
  or to the agent layer): the process DEFINITION the board engine runs (states +
  gates), the team-methodology specifics (iterations, verdicts, acceptance, roles),
  AND agent/process affordances baked into the substrate today — `precommit`,
  `speech_guard`, and kin. Fine things, just not the substrate's job.
- **Mixed — the `bridge/` agent-interface surface (see the agent-interface section):**
  the tools that *operate the board process* (`create_story`, `start_story`,
  `move_story`, accept/reject, `submit_verdict`, `view_board`, …) travel with the
  process; the coordination tools (`read_messages`/`ln`, send, presence,
  `get_identity`, `user_fn`) are substrate. **Channels GO** (terminal-injection
  replaced them); MCP tools undecided; CLI is the primary affordance.

`GitHub` lands entirely at the edge for BOTH `ci` (Actions) and `sync` (issues)
— that's stream D (push adapters to the edge). The substrate/process line is now
fully drawn.

## The agent interface (bridge) — and codebase hygiene

**The agent ↔ bpmux interface evolved; the code hasn't caught up.**
- **Old:** agents talked to bpmux over **MCP** (`bridge/tools`) + **claude channels**
  for real-time comm.
- **Now:** agents live in **terminals inside bpmux**; the interface is **CLI
  affordances** (which should mirror the MCP tools, or exceed them); channels are
  replaced by a **message injected into the agent's terminal input** (terminal-injection).
- **Decisions (owner):** **channels GO** (injection replaced them). **MCP tools:
  undecided** — keep for now; CLI mirrors them. CLI is the primary affordance.

**Cleanup targets (same substrate/process filter as the board):**
- **`bridge/` is unorganized + over-stuffed** → reorganize and slim: remove the
  channel machinery; reconcile MCP-vs-CLI duplication; the board-process-operating
  tools travel with the *process*, the coordination tools stay.
- **`src/` root has misplaced files** → re-home to their domains.
- **Agent/process affordances are NOT bpmux's responsibility** — `precommit`,
  `speech_guard`, and kin are agent/process concerns; lift them out (goal #2, the
  same "process leaves the engine" move as the board's process config). Not bad —
  just not the substrate's job.

**Why cleanup is load-bearing, not cosmetic (owner, 2026-06-25).** Long-context
work degrades organization — half-moved files, grab-bag dirs, names that lie. A
**misleading artifact is a bad prompt**: the next instance reads it and does the
wrong thing, and the mess compounds. So the codebase reading TRUE is a correctness
guardrail, not tidiness. ([[feedback_code_organization_is_an_llm_prompt]],
[[feedback_existing_code_is_not_the_spec]].) This cleanup is part of every stream,
not a separate chore — leave each file you touch reading true.

## The frontend — an untouched dimension that needs the same attention

⚠️ This plan was modeled entirely backend-first; **the renderer has had none of
the same treatment**, and that's a gap. The frontend (Electron + React) needs its
own architectural pass, equal to the backend's:
- **A clear state model** — how renderer state is held and updated; whether it
  mirrors the backend's Redux shape or has its own; no back-door mutation.
- **Wire ingestion on the typed contract** — the renderer decodes the wire; once
  the contract is codegen'd, ingestion is typed end-to-end. The conversation arc
  (last) reshapes this ingestion (`message.id` keys, `conversation_id` scope).
- **Organization + code-as-prompt hygiene** — the decompose sweep
  ([[project_bpmux_renderer_decompose_sweep]]); `fetch` only in `api/http`
  ([[feedback_frontend_fetch_only_in_http_module]]); the folder reads true.
- **The same modeling dialogue** — what the backend got this session, the frontend
  has NOT. Before #5 becomes concrete, model the renderer's architecture with the
  owner the way we modeled the store. Don't slot "do the frontend" as a task
  without that understanding.

The **typed contract** (above) is the seam: it lets the two halves evolve coherently
and is why conversations (which changes both sides at once) rides last, on the contract.

## The work streams (proposed sequence)

- **A. Replace the god-object `Store` with the agnostic kernel** (shape proven
  in `store_spike/`). Promote the spike's `kernel` (generic `Store<S, A>` +
  middleware chain + reducer + subscriptions) to the real store; strip the named
  domain fields (`ci_store`, `reactions_store`, …) out of `src/store/mod.rs`.
  Move each existing slice (`ci`, `reactions`, `roster`, `sync`) OUT of `store/`
  into its own domain module owning its `State`/`Action`/`reduce`/`middleware`.
  Compose them at the composition root (`AppState` + `AppAction` + `app_reduce`
  + the middleware chain). The structural heart — makes the store legible AND
  decouples it from every domain. Migrate one slice at a time, starting with
  `roster` (the spike already models it).
- **B. Name Redux into visibility (per slice).** Inside each domain's slice use
  Redux vocabulary: state, reducer, action, selector. For the board: `apply_events/`
  → `reducers/`; `orchestrator/` → the board's sync listener(s); `BoardStore` →
  the board slice. Owner opens a domain folder and the Redux roles are obvious.
- **C. One typed contract.** Generate the renderer's TypeScript wire types from
  the `view_model` structs (e.g. `ts-rs` / `typeshare`); delete the hand-written
  TS interfaces. Closes the front↔back drift. *(Independent of A/B/D.)*
- **D. Push adapters to the edge.** GitHub (and any external service) becomes an
  edge adapter, out of the domain core (hexagonal) — get `adapters/github/` out
  of the board.
- **E. Draw the substrate / process line + extract.** Per the decision above,
  move the AI-process domains (with their slices) into the light layer; leave
  bpmux a clean generic substrate. *(Largest stream; own decomposition.)*
- **F. Self-documenting structure.** Folder shape + a one-screen "what each slice
  is / what's next" pointer the next instance reads first (this file).

## Recommended order (dev + owner, 2026-06-25)

Sequencing principle: **order by reversibility + coupling, not by importance.**
Reversible/decoupled/enabling work first; irreversible/coupled/data-touching last.
(That is why conversations — the most important, the durable core — comes LAST:
it is the most irreversible (live-data migration) and most coupled (full-stack).)

1. **Hygiene** — BACKEND debris (dead channels, misplaced `src/` files, quarantine
   `precommit`/`speech_guard`) AND a FRONTEND legibility/organization pass (the
   renderer has its own debris — decompose sweep). Clears the ground both sides.
2. **Kernel** (stream A) — land the agnostic store; migrate ci/reactions/roster/sync
   onto it; finish it (no two-shape store).
3. **Story slice (the entity) + id-on-create** (streams B/D) — fold the story store
   (today's mis-named `BoardStore`) onto the kernel; persistence is `drain`/`ingest`
   effects over the `SourceAdapter` port (GitHub adapter at the edge); board becomes
   a *selector* over it. Deliver story birth-identity + optimistic-create-shows-its-id.
   First visible slice; touches the frontend lightly (renders the optimistic story).
4. **Typed front↔back contract** (stream C) — codegen TS wire types from the view
   structs. PROMOTED: it's the seam both sides build on, and #5/#7 need it.
5. **Frontend architectural pass** — give the renderer the same attention the
   backend got: a clear state model, wire-ingestion on the contract, organization,
   its own code-as-prompt hygiene. ⭐ NEEDS ITS OWN MODELING DIALOGUE — we modeled
   the backend all session and never the frontend; that's a real gap to fill before
   this becomes concrete.
6. **Process + agent extraction** (stream E) — board process → config; fully
   extract `precommit`/`speech_guard`; slim `bridge/`; MCP-vs-CLI.
7. **Conversations — LAST, the full-stack capstone.** Schema S2–S8 + messages slice
   with projection-persistence (S1 landed). WHY last: it is a DATA MIGRATION of the
   real durable artifact (most irreversible), and it reshapes FRONTEND INGESTION
   (`message.id` keys, `conversation_id` scope, retire to/addressee/privacy/
   BROADCAST_ALL shadows). Ride it on the typed contract + the cleaned frontend +
   the stable kernel — change the dangerous thing when everything it touches is settled.

## Status / next

### Session 2026-06-27 (dev, autonomous) — e2e safety net + #14 begun

**e2e safety net DONE (pushed green).** Built the visual-layer net as CLI
user-journeys (fire the real `bpmux` binary — the agent's control surface).
Shared harness `applications/electron/tests/helpers/bpmux-cli.ts`; journeys for
`rejected→Ready` + the `dev-accepted` badge. Audit confirmed every other
reachable `columnFor`/`deriveBadge` branch is already pinned; `ready-for-accept`
(no substrate producer) + the `pending` badge (deriveBadge default) are
unreachable dead branches → the codec refactor deletes them, no e2e fabricated.

**#14 codec-decode-first STARTED — slice 1 landed (`b4e39050`).** Per the
transition shape (§"Store structure" step 1), began moving label-decode from the
~40 read-time `*_from_labels` selectors INTO ingest-time. First field = the
routing key `story_type`: now a stored `StoryType` field on `Story`, decoded at
ingest (adapter `decode_story_metadata` = label-or-title; reducer/`new_minimal` =
title via the domain `story_type_from_title`), exposed via `Story::story_type()`.
Additive + behavior-preserving (field mirrors the selector). **Invariant that
makes reader-migration safe:** `s.story_type() == story_type_from_labels_or_title
(&s.raw_labels, &s.title)` for every story — the type label is immutable
post-creation (state/acceptor/blocked/position deltas never touch it; ingest
re-decodes), so the stored field never diverges from raw_labels.

**story_type field COMPLETE (origin @ d7fa42fe).** Stored on Story; all 36
readers across transport+bridge+domain migrated to `Story::story_type()`; only
the ingest setter (`decode_story_metadata`) still decodes. KEY CORRECTION mid-way
(the e2e net caught it): story_type is **authoritative-at-source**, NOT
label-derived — a story is created label-less (type in the `StoryCreated` event /
`[Epic]` title; the type label only reaches GitHub for the next ingest), so
re-decoding it from stored `raw_labels` on label change clobbered every epic +
optimistic iteration/outcome to Feature. Correct model: set it at creation
(event) + ingest (full labels+title), PRESERVE through label churn. The reverted
codec-decode-on-label-change mechanism (LabelDerivedFields carrying the decoded
field) WAS right — just for the wrong field.

**ALL SIX Story-shape fields DONE (origin advancing per slice):** story_type
(authoritative-at-source), then the label-derived acceptor, parent_epic,
iteration_membership, tags/blocked, ready_order — each decoded at the codec
(LabelDerivedFields → project_labels → reducer assign), exposed via a
`Story::field()` accessor, all readers migrated, and the wire RE-ENCODES every
Story-shape label from the decoded field (not from raw_labels). raw_labels is now
**vestigial for the Story shape** — only the sibling-type labels (iteration/
outcome/epic shape) and renderer-badge labels still pass through verbatim in
view_model. Tests realistic: the field-completeness sentinel asserts each decoded
field from its GitHub-JSON label; per-event store tests pin decode on the
label-add event; list_epic_children_tests builds via decode_story_metadata.

**NEXT — step 2: peel the sibling types (Iteration/Outcome/Epic).** The
substantial restructure (its own modeling pass — pace it). Surface map (Explore,
2026-06-27): ~25 routing readers branch on `story_type == Iteration/Outcome/Epic`
(mostly GUARDS in bridge/tools_stories: reject-if-not-shape); iteration-shape data
= iteration_order/review_closers/dev_ok/is_current selectors + IterationReadiness
(product_management/iteration/); outcome-shape = horizon/horizon_order/
outcome_membership selectors + OutcomeReadiness (product_management/roadmap/);
Epic is LIGHTEST — no forward data, just the type label + children pointing via
parent_epic (Story field) + derive_epic_counts/state rollup. Store is one
`HashMap<StoryKey, Story>`; single `decode_story_metadata`.
DECOMPOSITION (each CI-green, lower-blast-radius first):
1. Define `Iteration`/`Outcome`/`Epic` structs (shared identity/content + their
   shape-specific decoded fields) + a `BoardItem` enum
   (Story|Iteration|Outcome|Epic) — additive, with a decode-router (type label →
   variant, decoding that shape's labels) + tests. Not yet wired into the store.
2. Store value becomes `BoardItem` (one map — lower blast-radius than 4
   collections); ingest routes by type; `all_stories()` returns the Story
   variants (back-compat); add `iterations()/outcomes()/epics()` accessors;
   per-variant reducer arms.
3. Migrate consumers per shape to the typed accessors — OUTCOME FIRST (bounded:
   roadmap + a few tools), then ITERATION (readiness/current/members + the
   guards), Epic last (trivial). Each retires that shape's raw_labels selectors;
   the sibling fields leave raw_labels exactly as the Story fields did.
THEN P4/P5 identity-enum + id-on-create, Story-first.

#### CONVERGED DESIGN — the peel, via Designing-with-Types (owner + dev, 2026-06-27)

Owner chose (B) full per-shape types (NOT projection half-measures — they'd stay
green but leave the discriminant-blob that forced this refactor; structure IS the
deliverable, [[feedback_code_structure_is_the_prompt_for_decisions]]). Design lens
= Wlaschin **Designing with Types** ([[feedback_designing_with_types_north_star]]):
make illegal states unrepresentable; the type is the high-bandwidth prompt.

- **`enum BoardItem { Story(Story) | Iteration(Iteration) | Outcome(Outcome) |
  Epic(Epic) }`**; store = `HashMap<StoryKey, BoardItem>`. The DU kills the
  cross-shape illegal states the `story_type` discriminant ALLOWED (an "iteration
  with an acceptor", an "outcome in desk-check"). Generic events; `reduce()`
  routes by the stored variant (per-variant arms). Accessors `stories()/
  iterations()/outcomes()/epics()` replace `all_stories().filter(story_type==X)`
  — a consumer's accessor now declares which domain it touches.
- **Each shape = `Identity` (composition: id/title/body/filed_at/filed_by) ⊕ its
  OWN lifecycle ⊕ ONLY its fields.** Born honest (illegal cross-shape field-combos
  vanish for free — Iteration has no acceptor/parent_epic/tags/ready_order).
  - **Lifecycle is per-type (no shared `state`):** Story keeps `StoryState`
    (workflow). Iteration's lifecycle = its readiness gates (drafted →
    review-closed → dev-ok → building → closed). Outcome has **NO state field** —
    "achieved" is DERIVED from its building iterations (don't store the derivable).
    Epic has no stored state — derived from children.
- **Container owns ordered membership (the killer DwT move):** `Iteration { …,
  members: Vec<StoryKey> }` where INDEX = story order; `Outcome { …, iterations:
  Vec<IterationKey> }` = iteration order. The child carries **no position** (drop
  `iteration_membership`'s position; the just-landed Story field RE-HOMES to the
  iteration's Vec — step-1 got it off labels, the peel homes it to its owner).
  Single source of truth = the container's Vec; derive any child→container
  back-ref, never store a dual-state. This makes dup-position / gaps /
  orphan-position UNREPRESENTABLE → **deletes the `IterationReadiness.
  stories_ordered` gate** (it guarded exactly that illegal state; code disappears
  = the model is right). Epic owns no order (children by number). `ready_order` is
  the exception — a standalone ready story has no container, so its queue position
  stays a story attribute (its own question).
- **Adapter = parse-don't-validate:** GitHub still encodes per-child
  (`parent-iteration-#N.P` on each child issue) — wire-per-child ≠ domain-per-child.
  The codec/adapter RECONSTRUCTS each container's `Vec` from its children's labels
  at INGEST; a reorder mutates the one Vec; DRAIN re-encodes the children's
  position labels. The decoder is the one place a raw issue becomes a typed shape
  that thereafter guarantees its invariants.
- **File structure (first-class domains in the tree):** `product_management/
  {story,iteration,outcome,epic}/` each = its type + lifecycle + ops; `board/
  item.rs` (BoardItem) + `board/store/` (reduce routing) + `board/adapters/github/`
  (issue→shape decoders); `roadmap/` becomes a pure VIEW-composer over
  `outcomes()+iterations()` (stops being Outcome's home). `epic.rs` → `epic/`.
- **Sequence:** structural slice (BoardItem + the 4 honest types + Identity +
  store + ingest-routing + accessors + reducer arms) FIRST; then per-shape consumer
  migration (Outcome → Iteration → Epic); then Story's INTERNAL state-DU tightening
  (fold acceptor/observation/etc. INTO the Accepted/Delivered variants so
  "acceptor on an icebox story" is unrepresentable) as its own focused pass. DwT
  throughout. Owner wants architectural/algorithmic conversation as it proceeds.

**Refinements settled in the design conversation (owner + dev, 2026-06-27) — these
supersede where they touch the above:**
- **Identity = the key, AND lives in the object.** The shared struct is
  `Content { id: StoryKey, title, body, filed_at, filed_by }` (NOT "Identity"; not
  `number: u64` — the number IS the id, raw u64 is the adapter's edge-unwrap). The
  id is intrinsic + self-describing (a `BoardItem` tells you what it is on its own)
  AND is the map key. Consistency (key ≡ `item.id`) is held by the store API:
  `insert(item)` DERIVES the key from `item.id` (no `insert(arbitrary_key, item)`);
  immutable kernel = no back-door. Re-key on save = ONE operation (Pending(uuid) →
  Saved(number) updates the id field and re-files the map entry together). Don't
  store id-less + don't worry about the dual-state — it's maintained at the one
  insertion point (same discipline as single-parent below).
- **Iteration lifecycle is a DU; container owns order; single-parent via immutable
  assign.** `Iteration = Building { backlog: Vec<StoryKey> } | Completed { snapshot:
  Vec<StoryKey> }`. `Building.backlog` = the live ordered members (index = order; a
  finished story drops out). `Completed.snapshot` = the FROZEN shipped ordering
  (auditable history, inert). Container-owns-order makes dup-position/gaps
  unrepresentable AND deletes the `IterationReadiness.stories_ordered` gate. The
  single-parent invariant (a story in ≤1 iteration) is NOT type-enforced by the Vec
  (two Vecs could list a story) — it is maintained by the ONE pure `assign(state,
  story, iteration)` transition (drop-from-any + add-to-target, atomic on the
  immutable kernel) + completed-iterations being inert (the live competitor set is
  ~one Building iteration). This is the accepted DwT form: forbid the illegal states
  that matter, maintain the rest at a single construction point, keep objects
  self-describing — NOT strip-everything-to-forbid-every-dual-state. (Outcome
  symmetrically owns `iterations: Vec<IterationKey>`.)
- **Known compromise — generic events × per-type lifecycle:** a generic event
  carrying Story-specific payload (e.g. `StateTransitioned{StoryState}`) is
  nonsensical for an Iteration/Outcome but is representable (firable at their key);
  the reducer's per-variant arms absorb the mismatch (no arm = ignore). Accepted
  eyes-open for less churn; per-shape events would be the fully-honest form.
- **Open audit before claiming siblings are raw_labels-free:** confirm EVERY label
  is either decoded into a shape field or genuinely droppable (watch
  `is_renderer_badge_label` / `is_iteration_label_loose` verbatim-forwards).

**FINAL membership model (deeper design convo 2026-06-27 — SUPERSEDES the
container-only framing above):** the parent↔child relationships are **one-to-many,
not many-to-many** (an iteration has many stories; a story has ≤1 iteration AND ≤1
epic — "multiple parents" = multiple FK *fields*, one per parent type, each single).
The model is **bidirectional**:
- **FK on the child, one per parent type** (`Story.iteration: Option<IterationKey>`,
  `Story.epic: Option<EpicKey>`, `Iteration.outcome: Option<OutcomeKey>`) — the
  `Option` type-enforces single-parent-per-type, and gives O(1) "who's my parent".
  The just-landed `iteration_membership: Option<(u64,u32)>` SPLITS: the `u64` →
  the FK; the `u32` position → the container list. `parent_epic` → the `epic` FK.
- **Container owns the ORDER** as an ordered id-list (`Iteration.Building.backlog:
  Vec<StoryKey>`, `Epic.Active.stories`, `Outcome.Active.iterations`) — index = order,
  independent per parent.
- FK = membership (+ cap); list = order. Consistency (each list = a permutation of
  its FK-members) maintained at the one `assign`/`reorder`/`remove` transition on the
  immutable kernel; store resolution tolerant of a dangling id. Both invariants
  type-enforced at once (single-parent on the FK `Option`, no-dup-order on the `Vec`
  index). NOT many-to-many; no junction.
- (Lesson banked: don't mechanically import relational-normalization
  ("no repeating groups") into an immutable single-source domain — see
  [[feedback_designing_with_types_north_star]]; and pin cardinality with the owner,
  don't read it off the old `Option` shape.)

**LANDED 2026-06-27 — `StoryType` bifurcation (first peel slice, the precursor):**
`enum StoryKind { Feature, Bug, Chore }` + `enum StoryType { Story(StoryKind), Epic,
Iteration, Outcome }` — "an iteration with a kind" is now unrepresentable. Custom
`Serialize`/`Deserialize` on `StoryType` pins the FLAT capitalized string the OUTBOX
(`~/.bpmux/board-outbox.json`) already persisted (parse-don't-validate; a behavioral
test pins it so queued `StoryCreated` events from before the split still load).
`StoryType::as_label()` is the single source for the lowercase wire/GitHub string
(consolidated the codec + view_model + list_epic_children, which had been
Debug-lowercasing — that brittle pattern is gone). `Story::kind() -> Option<StoryKind>`
for KIND readers; SHAPE checks (`== StoryType::Iteration`) unchanged. 2391 backend
tests green, clippy clean. The shape variants (Epic/Iteration/Outcome) become the
`BoardItem` DU variants in the next (structural) slice; `StoryKind` stays on `Story`.

**DONE (uncommitted at time of writing) — Slice 1: Outcome carve-out (first shape
peeled, shape-by-shape).** The store is now `HashMap<StoryKey, BoardItem>` with
`enum BoardItem { Story(Story), Outcome(Outcome) }` (grows a variant per shape).
`Outcome = Content { id, title, body, filed_at, filed_by } ⊕ OutcomeLifecycle
{ Active { horizon, horizon_order } | Retired }` — born honest (sheds `raw_labels`;
horizon read via `.horizon()`). Ingest routes outcome-issues → `Outcome::from_story`;
the reducer routes by variant with **pure DwT transitions** (`retired()`,
`with_horizon_labels()`, `with_body()`, `with_title()` — variant-replacing, never
field-poking; story-only events no-op on outcomes via a `story_mut` helper). The
middleware reconstructs an outcome's horizon labels (`encoded_labels()`) so the
TagAdded/TagRemoved → label-delta machinery still drives horizon reprioritize.
`store.outcomes()` / `RefreshDriver::outcomes()` added; the wire **re-projects**
outcomes (`StoryView::from_outcome`, appended in `ordered_board_stories`) so the
board JSON is byte-unchanged; the roadmap read-path (`derive_roadmap_view(&[Outcome],
&[Story])`, `outcome_readiness(number)`) + the write-handlers (`ensure_outcome_in_
canonical_store`) consume the typed `Outcome`. 2393 backend tests green, clippy clean.
(A latent bug surfaced + fixed: `apply_title_updated`/`apply_body_updated` were
story-only, silently dropping outcome edits — now variant-aware.) NEXT: Iteration
slice (add `BoardItem::Iteration`, its lifecycle DU + readiness fields + the
outcome↔iteration ordered membership), then Epic.

### Overnight session 2026-06-26 → 27 (dev, autonomous) — state of play

**Landed + pushed green:** P1 (StoryKey storage seam — board store map keyed by
`StoryKey`); P2a (content read-accessors `title/state/body/filed_at/filed_by/
raw_labels/free_comments/gate_comments` + migrated `epic.rs`, `view_model.rs`,
the store filter); the **identity foundation** (`Story::id() -> StoryKey`,
`StoryKey` `Display` = `#42` / `(unsaved)`, `StoryKey::saved_number()` edge
unwrap); plus the plan/design refinements below.

**Design decided WITH you tonight (all recorded above):**
- `id() -> StoryKey` is the identity accessor — the number BECOMES the id; the
  raw `u64` GitHub issue-number is an EDGE concern (adapter / bridge / wire).
- For a STORY, open/closed = terminal lifecycle state (`Accepted`/`Retired`),
  DERIVED from `state`; the #1144 "second axis" is the **sync window**. So
  `is_closed` leaves the domain (adapter maps GitHub open/closed ↔ terminal).
- `body_etag` leaves the domain → the adapter's own etag cache.
- `raw_labels` leaves the domain → a **per-shape codec**; labels are
  shape-specific, each adapter decodes the ones its shape cares about; the type
  label is the routing key.

**Why I stopped where I did (didn't churn more code):** every remaining consumer
migration is entangled with `raw_labels` (FROZEN → per-type codec), `is_closed`
(→ adapter; behavior-risky to do blind, autonomous), or number-edge
classification — all gated on the **per-type split (#14)**. Migrating them now
would either rework against the split or risk a blind adapter change. P4 (the
enum) is also blocked: `StoryContent`'s shape depends on the raw_labels/per-type
decision. So I laid the safe identity foundation + captured the design rather
than build rework-bound or risky code.

**Recommended next (needs you): the per-type split modeling pass (#14)** — the
dominant remaining work. Decide each shape's exact fields + codec, the adapter
routing, store structure (per-type stores vs shared), and sequencing; then build
**Story-first**. That single decision unblocks `raw_labels` removal, P4, the
`is_closed`/`body_etag` lifts, and the rest of the consumer migration.

- Last landed on `main` (CI-green): board fold + `view_model` + the test-hygiene
  pass.
- **The architecture is validated** in `store_spike/` (uncommitted): the agnostic
  kernel, a domain-owned `roster` slice, the composition root, and the middleware
  chain (`api.dispatch`-to-top vs `next`-down), all typed and tested. Decide:
  keep the spike as the reference (commit under `store_spike/` or move to `docs/`)
  or delete once this plan captures it.
- **Stream A is the heart and is NOT blocked** by the substrate/process line.
  B, C, D can proceed independently. Only E (extraction) waits on the owner
  confirming the line.
- Proposed first move once the owner okays direction: **stream A**, migrating
  `roster` first (it's already modeled in the spike).

## How this directs the next instance (READ THIS FIRST, then move)

You were cleared/restarted on purpose — long context degrades this kind of work.
**This file is your context now.** The decisions in it were made with the owner
across a long session, each with its rationale recorded above.

**These are SETTLED. Extend them; do NOT re-litigate them.** If you believe one is
wrong, say so and raise it — but never re-open a settled call without NEW evidence
([[feedback_dont_ask_iterate_experiment]], [[feedback_existing_code_is_not_the_spec]]).
Settled (the WHY is above — don't re-derive it): agnostic Redux kernel; slices own
their state; AI workflow (process/names/rules) = per-instance config; conversations
are the durable core and go LAST; story persistence = Ports & Adapters + Transactional
Outbox + CQRS projection (NOT "sync"/"store"/"board"); board = a VIEW over stories;
horizon = a view; total identity from birth; and the ORDER below.

**Start now, no ceremony:** the "Recommended order" list, top to bottom.
- **#1 hygiene.** First concrete, lowest-risk action: remove the **dead channels**
  (terminal-injection replaced them). Then re-home misplaced `src/` files. Then
  quarantine `precommit`/`speech_guard`. Each = its own CI-green, **path-limited**
  commit (shared tree — commit ONLY your paths;
  [[feedback_shared_working_tree_path_limit_commits]]).
- Then #2 kernel, #3 story slice + id-on-create, and so on.

**Pause for the owner ONLY at:** #5 the frontend pass (explicitly UNMODELED — needs
its own design dialogue, like the backend got), and any genuinely-new fork. Otherwise
execute autonomously, CI-green per slice ([[feedback_soft_wait_dual_gate_terminal]]).

Work the list. Don't re-plan it.
