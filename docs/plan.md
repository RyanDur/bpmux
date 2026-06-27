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

## Status — where we are

Updated 2026-06-27. (Per-slice history + detailed design live in git + dev memory;
this section stays the high-level position.)

**Done + on `origin/main` (CI-green):**
- #1 hygiene; the agnostic Redux kernel (stream-A foundation); board-store folded
  onto the kernel; the `is_closed`-leaves-domain + id-on-create-prep slices.
- Story identity: P1 (StoryKey storage seam) + P2a (content read-accessors) +
  `Story::id() -> StoryKey`.
- e2e safety net — CLI user-journeys covering the board's visual states.
- **#14 slice 1 — `StoryType` bifurcation** (`StoryKind {Feature,Bug,Chore}` +
  `StoryType { Story(StoryKind), Epic, Iteration, Outcome }`) — `1b059fe3`.

**In progress — #14 sibling-type peel — PAUSED (architectural pivot; see Notes):**
- The Outcome-carve store work is committed `f86c5731` but **UNPUSHED + HELD**. Its
  domain half (`BoardItem`/`Outcome` types, per-shape reducer, per-domain decode) is
  keepable; its wire/board half (`StoryView`/`ordered_board_stories` re-projection)
  is **superseded** by the backend-domain/frontend-view pivot.

**Blocked on:** an owner planning pass for the #14 re-approach (per-domain adapters →
domain collections → frontend-owns-view — see Sub-plans). Also gated on #14:
`raw_labels` removal, P4 (the Story DU), the `is_closed`/`body_etag` lifts, the rest
of the consumer migration. **No further #14 code until the re-approach is planned.**

## Notes — decisions & directions

SETTLED architecture (extend, don't re-litigate without NEW evidence —
[[feedback_existing_code_is_not_the_spec]]): agnostic Redux kernel; slices own their
state; **one store per domain**; AI workflow (process/names/rules) = per-instance
config; conversations are the durable core (go LAST); story persistence = Ports &
Adapters + Transactional Outbox + CQRS projection; total identity from birth.

Directions established 2026-06-27 (detail in the linked memories):
- **Backend = domain, frontend = view.** The wire serves DOMAIN per-type collections,
  not a board/presentation; **no "orchestrator"** (dissolved by the kernel-fold — its
  jobs are reducers/middleware/listeners) and **no backend "board"** (the frontend
  owns its view). [[feedback_backend_domain_frontend_view]]
- **Per-domain ports + adapters** — each PM domain's adapter queries GitHub for its
  issue-type by label and decodes to its type; the shared client is hidden, so each
  reads as a separate integration. [[feedback_github_at_edge_hexagonal]]
- **Designing-with-Types** — illegal states unrepresentable; DU/record/Option/
  parse-don't-validate/lifecycle-as-DU. [[feedback_designing_with_types_north_star]]
- **Code structure is the prompt.** [[feedback_code_structure_is_the_prompt_for_decisions]]
- **Identity** = `id() -> StoryKey`; the raw `u64` is an edge concern. `is_closed`
  (→ derived from state), `body_etag` (→ adapter cache), `raw_labels` (→ per-shape
  codec) all LEAVE the domain.
- **Membership** = one-to-many: FK on the child (the cardinality cap) + the container
  owns the ordered list (the order); bidirectional, kept consistent at one transition.
- SUPERSEDED: the earlier per-shape `BoardItem`-DU "CONVERGED DESIGN" + the
  backend-board wire re-projection — replaced by the backend-domain/frontend-view
  pivot (git history holds the detail).

## Sub-plans — in-flight & queued work

### #14 — roadmap shapes as distinct domain types (re-approach, post-pivot — TO PLAN)

Target: the backend exposes the PM domains via **per-domain ports + adapters** (each
a label-scoped GitHub query → its type), serving **per-type domain collections**; the
frontend builds columns/bands/roadmap. No backend board; no late `story_type` filter.

Cut **shape-by-shape** (Outcome → Iteration → Epic); each shape = one honest vertical:
`type + port + GitHub adapter + store collection + domain-collection wire + frontend
reads it + retire that shape's late-filter`.

- Open question to resolve first: do any derivations (iteration readiness, ready-order)
  stay server-side (shared by the CLI / workflow signals), or are they pure view?
- Reuse: `f86c5731`'s domain types (`Outcome`, the per-shape reducer) are a likely
  starting point; its wire/board half is discarded.
- Then: P4/P5 identity-enum + id-on-create (Story-first) once the shapes/codec settle.
- Status: **PAUSED pending the owner planning pass.** Detail + the design history live
  in dev memory ([[feedback_backend_domain_frontend_view]],
  [[feedback_designing_with_types_north_star]]) + git.

### Story identity (P-series — interleaves with #14, gated on the per-domain decision)

P1 (StoryKey seam) + P2a (content accessors) + `id()` foundation DONE. Remaining:
P2b+ (lift `is_closed`/`body_etag`/`raw_labels` to derivation/adapter), P3 (construction
seam), P4 (`Story` Unsaved/Saved DU + `StoryContent`), P5 (id-on-create Pending→Saved),
P6 (renderer optimistic story). All gated on #14's per-domain decision (the
shapes/codec/store structure they depend on).

## How this directs the next instance (READ THIS FIRST, then move)

**This file is your context.** Read **Status** (where we are), **Notes** (the settled
architecture + the 2026-06-27 directions — extend, don't re-litigate without NEW
evidence; [[feedback_existing_code_is_not_the_spec]]), and **Sub-plans** (the in-flight
detail), then continue down "Recommended order".

**Current position:** #14 (roadmap shapes → distinct domain types) is **PAUSED**,
pending an owner planning pass on the per-domain-adapter / domain-collection-wire /
frontend-owns-view re-approach (see Sub-plans). Do **not** resume #14 code, and do
**not** push `f86c5731`, until that re-approach is planned with the owner.

**Pause for the owner at:** the #14 re-approach planning; the frontend pass (its own
design dialogue, like the backend got); any genuinely-new fork. Otherwise execute
autonomously, CI-green + **path-limited** per slice (shared tree —
[[feedback_shared_working_tree_path_limit_commits]], [[feedback_soft_wait_dual_gate_terminal]]).

(Reference: the kernel architecture is validated in `store_spike/`; keep design
churn + per-slice history out of this doc — those live in dev memory + git.)
