# Architecture rules — state scopes, reads, writes

Status: converged with Ryan 2026-07-04 through the competing-architecture-ideas
talk (grounded in three firsthand substrate sweeps: write paths, derived-read
homes, paradigm stragglers). This is team ground truth for WHERE state lives,
WHO derives, and WHAT reacts. It resolves the views/redux/events competition.
Sister docs: `board-projection-model.md` (the domain model),
`app-continuum-restructure.md` (module structure).

## Identity (must survive compaction)

bpmux is a peer system with no central server of its own. Actions are local to
the machine; instances never talk to each other directly (a shared chat surface
is planned — the one intended network flow). Instances meet only through GitHub.

> **GitHub stores what the team agrees on; each instance stores how it works.**

## The three scopes of state

1. **Shared durable — GitHub.** Stories and backlog artifacts: coarse workflow
   position, order (reference lists), comments/gate history, titles/bodies.
   The only shared state in the system.
2. **Instance-local.** The redux store (a working copy synced with GitHub, not
   a replica of any peer), the workflow strategy (`workflow.toml`), the
   in-progress organization and each story's fine-grained middle placement,
   verdict evidence, UI state, local persistence.
3. **Shared surface (planned).** Chat/presence — ephemeral, connection-driven,
   never stored centrally. (Open: which plane chat's persistence belongs to.)

## The board's shared vocabulary (Ryan, 2026-07-04)

The board has exactly three sections that mean the same thing on every
instance: **ready → in-progress → acceptance** (plus the off-board terminals
accepted/retired). Ready and acceptance are stable across instances. **The
organization INSIDE in-progress is instance-local strategy** — the
`workflow.toml` middle stages are this instance's private way of working, not
team vocabulary. Sharing in-progress strategies is a possible future, not now.

Two grains of story position follow (terms: a shared **section**; local
**sub-sections** within it):
- **Shared coarse position** (GitHub label): `ready | in progress | acceptance`
  + terminals. Every instance agrees. The label names the SECTION, never a
  sub-section.
- **Local fine position** (which sub-section of In Progress): this instance's
  business only. Today's local strategy: two sub-sections, `in progress` →
  `desk check` (entry = `in progress`). NOTE the name collision between the
  In Progress section and its entry sub-section: it camouflaged the leak —
  BOTH sub-sections encoded their own labels; the entry one's label was
  byte-identical to the section's, so only desk-check's leak was visible.
  Sub-section names are local-only post-fix, so renaming the entry
  sub-section away from the collision is free if ever wanted.

**Decisions (Ryan, 2026-07-04):**
- The `desk-check` GitHub label is a LEAK of local strategy into the shared
  store. GitHub carries only `in progress` for the whole middle. Migration:
  stop encoding stage-specific middle labels; decode `desk-check` tolerantly
  during transition (→ in-progress + local desk-check placement).
- Fine-grained middle placement persists in the instance's db/storage — a
  refinery-migrated table behind a per-feature repo port (the storage-ports
  pattern), hydrated at ingest to compose with the coarse GitHub fact.
- **The board is a DOMAIN (Ryan, 2026-07-04): the instance's working
  arrangement of the shared work.** The instance-local middle creates
  board-owned state — the placements (`StoryId → StageId`) — so the board
  returns as a proper slice: placements + local strategy rules + selectors
  composing the two grains. `Story.position` carries ONLY the shared coarse
  vocabulary; placements live in a type the SourceAdapter never sees — the
  desk-check leak becomes UNREPRESENTABLE, not codec discipline. Movement
  within the middle is a board action (local, no outbox, no story event);
  crossing a bookend is a story event (shared, gated). Placement exists iff
  coarse position is in-progress. NOT the evaporated god-module returning:
  S1–S8 dissolved codecs/mappers/view-models/sync wearing the board's name;
  this slice owns the domain content that module never had. Gate comments may
  name local stages (narrative, append-only) — labels may not (state, must
  converge).

## Reads: selectors derive, surfaces format

> A **selector** is a named pure function over the one store's state, owned by
> the domain that owns the question. Every surface — renderer, MCP tool, CLI,
> REST, WS snapshot — consumes selector output and may only SHAPE it (text,
> JSON, casing, TSX). The moment a surface computes a fact, it has stolen a
> selector.

- Selectors answer a question with JUST ENOUGH — bounded, capped, shaped — so
  agents and humans are equal consumers and no consumer is token-flooded.
- `crate::view` as a derivation home was drift (born in the board evaporation);
  derivations migrate into domain selectors. What remains at the surfaces is
  wire/text/UI formatting.
- Selectors split by scope: shared-fact selectors (ready set, acceptance set,
  backlog columns, order) eventually agree across instances; local-fact
  selectors (middle grouping) legitimately answer per-instance.
- The renderer's parallel TS derivation stack is a divergent hand-copy of
  selector logic (15 duplicated derivations, 4 live drifts at audit time). End
  state: the wire ships selector OUTPUTS and the renderer formats. Until that
  deliberate wire reshape, the mirror is tolerated, known debt.
- Display labels ("IN PROGRESS" vs "In Progress") are per-surface presentation
  and are ALLOWED to differ; facts are not.

## Writes: events write, state triggers

> Nothing inside the system makes a decision from an event. Effects, agents,
> views, broadcasts — all subscribe to state through selectors. An event's
> only internal consumer is the reducer.

- **Intents are atomic.** One intent = one action, applied atomically by the
  reducer — never a multi-dispatch event sequence a subscriber can observe
  half-applied. (The verdict collapse (A2) builds exactly this for
  transitions.)
- **Triggers are level, not edge.** An agent acts because state says there is
  work ("in desk-check, lacking my verdict"), not because a notification
  arrived. Crash-safe, idempotent, cause-agnostic. Completion markers must be
  DOMAIN facts (the verdict); if a completion marker would be mere effect
  bookkeeping, it lives in the effect's own slice, never on a domain entity —
  and that case should be rare (test: is this a product fact or the effect's
  memory?).
- **One judge per state is a load-bearing invariant** — it is the dedup
  mechanism for at-least-once triggering. The connection-driven roster is its
  enforcement point; one active holder of a role at a time.
- **Push = selector outputs.** Cross-process wake-ups (WS to the renderer and
  agents) carry selector output snapshots; diffing is an optimization to adopt
  only under measured payload pressure. Hand-authored per-action delta
  payloads are the drift-prone shape being retired.
- Events that remain, and why: the write vocabulary into the reducer (humble,
  internal); append-shaped domain data (gate/free comments — history is data,
  not derivable from current state); the outbox operations to GitHub.

## The GitHub edge

- **Create blocks until done** — identity (issue numbers) is the one operation
  that genuinely needs the shared store; the single deliberately-CP point.
- **Updates are optimistic + eventual**: reducer applies locally, the outbox
  queues the operation, the drain pushes async, ingest reconciles. Last-write-
  wins arbitration via `(ts, peer_id)` when instances collide through GitHub.
- **Everything behind the SourceAdapter port, after the reducers,** triggered
  off outbox STATE (a reconciler: outbox non-empty → drain, per-story ordered,
  create-blocking), not off command-actions.
- **Under LWW there is no conflict, only ordering (Ryan, 2026-07-04).** The
  later `(ts, peer_id)` write supersedes; instances converge; GitHub's own
  edit history is the recovery ledger for superseded prose. What remains is
  correct arbitration — `event_wins` ordering + own-echo recognition at
  ingest — not conflict machinery. `ConflictDetected`/`ConflictResolved` as
  concepts retire (execution verifies their consumers before deletion).
- **Writes return optimistically; pending-ness is selectable state (Ryan,
  2026-07-04).** The outbox IS the pending-set: `pending_operations_for(story)`
  is a selector every surface formats its own way (renderer not-saved badge;
  tool responses carry pending status). A row's removal at push-time clears
  it; a permanent push failure lands in `dropped_events` as the save-failed
  record (`MutationConfirmed`/`MutationRolledBack` retired 2026-07-07/08 —
  LWW reconciliation made per-mutation confirm/rollback signals redundant).
  Consequences: inline
  `drain_pending` calls go; body/title write-through folds into the outbox;
  no write verb blocks on GitHub (create excepted — identity).
- **The middle is fully silent externally.** Middle states are not broadcast;
  other instances see only In Progress. `pass_desk_check` is unaffected (it
  crosses the acceptance bookend — shared, gated, evidenced). The `Delivered`
  gate comment becomes local with the board domain; any "delivered" chat
  announcement is team process, not architecture.

## Violation ledger (audit 2026-07-04; addressed as gated refactor slices)

Already dissolved by the verdict-collapse arc: `verdicts.json` third substrate
(A2 moves the verdict onto the domain plane — it is peer-local today, invisible
to gates elsewhere); bridge transaction-scripts (Phase 1); `is_closed` /
`ready_order` shadows (C2/C1).

New, refactor-safe: selector consolidation (view→domain selectors; stories
currently have NO selector); the `!complete` current-iteration drift
(`transport/rest/board.rs`); state-label 3-homes classification; shadow-field
sheds (`Epic.state` SHED (this slice); `iteration_slot` SHED (slice D);
`story_type` KEEP — surveyed 2026-07-07: it is the canonical decode-once home
for a story's kind (labels-over-title at `story_decode.rs:62`), read by live
gate-class/signal-routing/chore-guard/ATP logic, pinned on the wire and outbox
deliberately; retiring it would reintroduce multiple-authority re-derivation);
drain trigger action→state; `EventMeta` envelope made structural (the 26-arm
double-match); fitness locks for unowned seams (bridge→*, sync→store,
integrations→store, view scope); reactions dual-write.

Decided, not behavior-preserving at the shared edge (owner-approved
2026-07-04): the desk-check label de-leak + local middle placement storage.

Found at C2 execution (2026-07-04) — **`is_closed` encodes TWO concepts, only
one a shadow**: closed non-accepted STORY → retired (shadow of
`position ∈ {accepted, retired}`; sheds via A2's handler recast) vs **closed
ITERATION → icebox/badge/drag-rejection — a CONTAINER-lifecycle fact whose
domain home already exists: `Iteration.complete`.** The renderer re-derives
container completeness from `gh_state` on the iteration's story-shaped
marker (a container fact read off a story wearing a costume — the
label-decorated-container disease board-as-slice-reshape retires). Re-home in
SC. C2 as a standalone slice was falsified by this + `gh_state` being on the
consumed wire; dissolved into A2 (backend re-expression) + SC (field/wire
shed, columnFor retirement, iteration-closed re-home).

New from the 2026-07-04 close-out: velocity is a chat-text parse
(`view/velocity/mod.rs:10` substring-scans announcements — phrasing-fragile,
wrong source). WHY it reads messages (introducing commit `55f3bb69`): the
anti-confabulation principle — "every number traces to an observable bpmux
signal — no self-report" — and at the time the chat log (system-broadcast
transition announcements) was the only local timestamped lifecycle record.
The PRINCIPLE survives the re-sourcing; only the substrate upgrades (prose
mirror of events → the events themselves). It becomes TWO honest selectors split by grain: shared metrics
(accepted-per-window, bookend timings) off story/verdict history — identical
on every instance; local metrics (pickup-to-deliver, desk-check rework) off
board-domain history — per-instance BY DESIGN. Also retired by decisions
above: conflict-surfacing machinery; inline `drain_pending`; the write-through
outbox bypass.

**Metrics (converged 2026-07-04):** two families, NEITHER message-based.
- **Velocity = Pivotal Tracker semantics: count of accepted business-value
  stories per iteration, each worth 1, rolling average.** Points mean nothing
  to velocity (Ryan); no estimate field enters the domain; the #251
  features-only filter stays. Shared-plane derivation (acceptance transitions
  × iteration windows) — identical on every instance.
- **Flow diagnostics (DORA-shaped):** lead time pickup→accept + acceptance
  rejection rate from SHARED history; sub-section timings + desk-check rework
  from LOCAL board history — per-instance by design.
- History becomes a first-class domain requirement: the store captures
  transitions at write time; GitHub gate/verdict comments are the durable
  source, BACKFILLED on cold start (thin metrics until backfill completes —
  accepted). The anti-confabulation guarantee (every number traces to an
  observable signal) rides the history's timestamps/actors; the chat-parse
  velocity (`view/velocity/`) retires.

**Closed at close-out:**
- Chat's plane: no current forcing function (velocity was the only converged
  consumer pulling messages toward the store, and it left). Chat stays as-is
  (local SQLite + broadcast; the planned shared surface) until a real need.
- `session.roadmap` / `RoadmapOutcomeCard`: NOT dead weight — a half-finished
  migration toward this architecture (backend ships the projection; renderer
  ignores it and re-derives in TS). Decision (Ryan): **finish it** — renderer
  consumes `session.roadmap`, the TS mirror (`selectRoadmap`/`parseOutcomeWhy`/
  `readinessStateOf`/`toneOf`) is deleted. Parity-check before the switch
  (unguarded pair, may have drifted; Rust is truth unless TS behavior is the
  wanted one). No wire change; Roadmap tests stay green unchanged. The first
  concrete instance of wire-ships-selector-outputs. **The hooks stay (Ryan):**
  `useRoadmap` (and the hook family generally) remains the renderer's
  consumption seam, backed by shipped selector outputs — only the derivation
  inside dies; components never change. This is the general pattern for every
  future mirror-deletion.

**The session renames (Ryan, 2026-07-04).** "Session" is a dissolved concept
whose name outlived every referent — four unrelated things wear one dead word,
which kept resurrecting the dead idea in readers. Rename each referent for
what it IS; the word leaves the codebase:
- renderer synced-state slice `session` → **`state`** (`state.roadmap`,
  `state.outcomes`; it IS the instance's state, received). Synergy: the
  remodel already renames the story's fact toward `position`, decluttering
  the word "state" as this lands.
- backend `Session` struct → derive from its responsibilities at execution —
  it is the instance's delivery **hub** (append to storage + fan out to
  connected users), NOT "the instance."
- `session_dir` → **`data_dir`** — where the instance's local durability
  projection lives; `BPMUX_SESSIONS_BASE` → **`BPMUX_DATA_BASE`**, an
  instances/data-dirs base (rename travels with its test-harness consumers).
- Mechanical, behavior-preserving slices; any wire-visible key that says
  "session" needs the wire check before renaming (internal renames first).

Related clarity (state vs persistence): **state lives in the store, only
there.** The local db and GitHub are the SAME pattern at two scopes — durable
projections written by listeners, read only at boot/backfill (hydrate), never
at read-time. Selectors read the store; code that queries persistence to
answer a question is treating persistence as state (same disease as a surface
deriving facts). Standing exception, deliberate: the chat log (append-only,
SQLite-direct, closed as stays-as-is).

**Twelve-factor pass (Ryan's standing lens; run 2026-07-04):**
- **Env-config vs instance strategy are different things — never share a
  mechanism or a home.** Env holds deploy-varying resources (GitHub repo,
  credentials, ports). `workflow.toml` is NOT config in that sense — it is
  instance STRATEGY (domain-adjacent data, heading toward board-domain
  ownership). The `current_workflow()` ambient-singleton fix must not
  "comply" by moving strategy into env — that would be the wrong direction
  dressed as Factor III.
- **Sudden death is a supported input (Factors VI+IX, intent-translated).**
  bpmux deliberately violates the letter of stateless-processes — the store
  IS in-process state, correct for a local instance app. The compensating
  invariant: the store must be completely rebuildable from its durable
  projections (GitHub hydrate/backfill + db hydrate) at ANY kill-point.
  Every write-behind persist listener defines a loss-window whose
  acceptable-loss is CHOSEN per slice, never inherited. The outbox already
  complies (flushes to disk inside `mutate`, before ack); the board-domain
  placement listener must make its call explicitly at design time
  ("lose the last drag on crash: yes/no"). Fitness-testable in spirit:
  kill mid-write, reboot, assert convergence.
- Factor XI (logs are event streams, not a database) is the named principle
  behind the velocity re-sourcing — an app mining its own announcement
  stream for domain data was the violation.

## What this retires conceptually

`crate::view` as a derivation home · action-keyed listeners (`SyncAction`
command-actions that reduce to no-ops) · hand-authored `StateDeltaPayload`
authoring as triggers · event notifications as agent triggers · the idea that
the store is replicated peer-to-peer (it is a working copy synced through
GitHub, nothing more).
