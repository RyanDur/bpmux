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
