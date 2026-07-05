# Sequencing plan — board-projection remodel (#69) + architecture-rules execution

Companion to `board-projection-model.md` (the domain model) and — GOVERNING for
every rule referenced here — `architecture-rules.md` (state scopes, selectors,
events, the GitHub edge; converged with Ryan 2026-07-04). Restructured
2026-07-04 after that convergence: the original A/B/C/D ordering survives
inside a larger queue that also carries the architecture ledger's slices.

**Guardrails per slice — the refactor constraint (Ryan, 2026-07-04):** every
slice is behavior-preserving (performance gains excepted); the app works as-is
throughout. The ONLY exceptions are the owner-approved shared-edge changes
named below (the desk-check de-leak; the velocity metric redefinition), each
marked ⚠. Each slice gated (nextest + vitest + e2e) → commit → push → verify
ls-remote; CI-green gates the next pickup. The refactor falsifier wherever a
slice claims pure re-expression: zero edits to existing tests.

## Landed

- **A1** ✅ `1685ab96` — ordinal apparatus deleted; `most_recent_needs_work`
  seed in `story/rework.rs`; both consumers re-expressed.
- **B1** ✅ `e241611d` — `ColumnBucket` dissolved into
  `view/board/arrangement.rs` (`board_position` / `board_columns` /
  `group_by_position` / `column_header`); wire byte-identical.
- Doc regrounding ✅ `2f153ce4`.
- **T0** ✅ `d4776f01` — backlog projection re-homed as a pm selector
  (`product_management/backlog.rs`, self-contained fold — does NOT reach into
  `view::board::arrangement::board_position`; C2 consolidates the transient
  duplication when `is_closed` sheds). Unbroke the red tree; landed the riding
  architecture-convergence doc edits.
- **RF** ✅ — roadmap finish. `selectRoadmap` re-pointed to consume the shipped
  `session.roadmap` cards + a join to `outcomes` for drill-in iterations; the TS
  mirror (`parseOutcomeWhy` / `readinessStateOf` / `toneOf`) deleted (Rust
  `view/roadmap_tests.rs` is the tested truth). `useRoadmap` + `Roadmap.tsx`
  unchanged (hooks stay, derivation dies). Owner call (Ryan, 2026-07-04): the
  authoritative `state` (`done|ready|needs-attention|planned`) is now on the
  wire, but the chip maps `done → ready` — a distinct "done" treatment is a
  future designer-owned UX slice. The e2e specs stayed green unchanged (the true
  falsifier); RTL fixture seeding switched from an `outcomes` delta to `roadmap`
  cards (the wire the renderer now consumes).

## Order at a glance

`T0` ✅ → `RF` ✅ → `BD1–BD3` (the board domain + de-leak — NEXT PICKUP) →
`A2` (verdict collapse, two-grain-native; absorbs the handler cleanups AND
C2's story-terminal re-expression) → `A3` (delete `Gate`) → `M1–M4` (metrics
re-sourcing) → `C1` (shed `ready_order`) → `D` (order-on-lists) → `SC`
(selector consolidation sweep + locks; absorbs C2's field+wire shed and the
iteration-closed re-home) → `WR` (write-path remainder) → `SR` (session
renames, last — mechanical, least churn).

C2 no longer appears as a standalone slice — it was **falsified as scoped**
(2026-07-04, mid-execution substrate check) and dissolved three ways; see the
C2 section below for the record.

Rationale: the board domain must exist BEFORE A2 so the verdict collapse is
designed two-grain-native rather than retrofitted (BD-before-A2 also means
the executor arrives at A2 with the grains fresh); metrics need the histories
A2+BD create; the renames go last to avoid churn.

## T0 — B2 as a product_management selector [unbreaks the tree]

The backlog projection is a DOMAIN SELECTOR (per architecture-rules: selectors
derive, surfaces format), not a `crate::view` module. Re-home the parked WIP:
- `product_management/backlog.rs` (or beside the story slice — dev judgment;
  NOT `view/`): `BacklogColumn { Icebox, ReadyForWork, Current, Accepted }`;
  `backlog_column(story, iterations) -> Option<BacklogColumn>` (None =
  archived/retired); `group_by_backlog_column`. Derivation rules and edge
  cases exactly as pinned in the parked `view/backlog_tests.rs` — move +
  re-point the tests, keep their behavior names.
- Rules in order: `board_position` fold → Accepted→Accepted → Retired→None →
  current-iteration member ∩ ready…acceptance→Current → any-iteration→
  ReadyForWork → else→Icebox.
- Remove the `view/mod.rs` backlog registration; register in pm instead.
- Zero consumers change; the bridge `view_backlog` JSON stays byte-identical.
- This commit also lands the riding doc edits.

## RF — finish the roadmap migration [self-contained; no wire change]

Backend already ships `RoadmapOutcomeCard`s (`view/roadmap.rs:68`) on every
snapshot; the renderer ignores them and re-derives in TS. Finish it:
1. Parity-check TS `selectRoadmap` output vs the shipped cards field-by-field
   (`name/why/horizon/state/gap_count/tone`, ordering). Rust is truth unless
   the TS behavior is the wanted one — reconcile deliberately.
2. `useRoadmap` returns the shipped cards. **The hooks STAY** — the hook
   family is the renderer's consumption seam, backed by shipped selector
   outputs; only the derivation inside dies; components never change.
3. Delete the TS mirror (`selectRoadmap`, `parseOutcomeWhy`,
   `readinessStateOf`, `toneOf`) + its tests.
4. Falsifier: Roadmap RTL/e2e tests stay green UNCHANGED (same surface,
   different source).

## C2 — FALSIFIED AS SCOPED; dissolved three ways (2026-07-04)

The original scoping ("backend-only, refactor-safe, ~34 files") broke on a
mid-execution substrate check: (a) `is_closed` is ON THE WIRE —
`StoryView.gh_state` (`view/board/story.rs:28`), consumed by the renderer's
`columnFor` and `isClosedIterationMarker`, so a shed is renderer-coordinated,
not backend-only; (b) the field encodes TWO concepts, only one a shadow —
closed non-accepted STORY → retired (a true shadow of
`position ∈ {accepted, retired}`) vs closed ITERATION → icebox + badge +
inbound-drag rejection (a CONTAINER-lifecycle fact, not a story position);
(c) real blast radius ~54 files incl. the renderer stack; (d) the renderer
consumption is exactly SC's `columnFor` mirror. Dissolution:
- **Story-terminal re-expression → folded into A2** (the read sites are the
  acceptance/reject/desk-check guards, `move_story`, and transition handlers
  — the very files A2 recasts; doing it earlier is pure double-churn).
- **Field + wire shed (`is_closed` off Story, `gh_state` off the wire) →
  folded into SC**, where the `columnFor` mirror retires deliberately. Until
  then `is_closed` is a documented, tolerated shadow.
- **The closed-iteration concept → its own fact with an EXISTING home:
  `Iteration.complete`.** The renderer today re-derives container
  completeness from `gh_state` on the iteration's story-shaped marker — an
  iteration fact read off a story wearing a costume. Re-home in SC (renderer
  reads the container); any visible change to closed-iteration rendering is
  designer-coordinated. Also further evidence for board-as-slice-reshape
  (containers as real wire entities, not label-decorated stories).

## BD — the board domain (three slices)

Per architecture-rules: the board is the instance's working arrangement.
`Story.position` carries ONLY the shared coarse vocabulary; placements live
in a type the SourceAdapter never sees.
- **BD1 — the board slice.** New top-level `board` feature module (sibling to
  `ci`/`roster` — NOT `product_management/board`; a proven-to-falsify fitness
  lock asserts that path stays deleted). Slice owns placements
  (`StoryId → StageId`), entry-sub-section default, invariant: placement
  exists iff coarse = in-progress. Within-middle moves become board ACTIONS
  (local — no outbox, no story event); selectors compose the two grains so
  every existing surface renders identically. Falsifier: board render + e2e
  unchanged.
  - **BD1a (EXPAND) — DONE** (`6a622020` `aece8a02` `4b79dfcb` `16d272b0`):
    board slice foundation; reconciler derives placement from
    `story.state == InProgress(stage)`; composing selector `board::story_position`
    with own-stage fallback; readers thread the composed position.
  - **BD1b (CONTRACT) — `BoardState::InProgress(StageId)` → unit; the fine
    stage lives ONLY in the board slice.** The channel that carries the fine
    grain today (inside `InProgress(stage)` on `Story.state`) closes, yet THREE
    facts stay byte-identical: the `/api/board` wire `state` field
    (`"desk-check"`), the OUTBOUND GitHub label (`"desk-check"` — the de-leak is
    BD3, and BD2 persistence isn't landed to recover it on re-ingest), and the
    board columns. The outbound label is computed from the EVENT's `new_state`
    at drain (`integrations/github/mutation_events/lifecycle.rs::push_state_transitioned`,
    race-free vs. the async placement listener), so the fine grain must ride the
    EVENT, not be looked up from the store at drain time. Decomposed
    expand/contract WITHIN the contract — migrate every reader and writer onto
    placement while `BoardState` still (redundantly) carries the stage, so the
    final type-flip is mechanical:
    - **BD1b-1 (reads → placement).** `board::fine_stage(story,board)->Option<Stage>`
      + `RefreshDriver::story_fine_stage`. Repoint every FINE reader off the inner
      stage onto `fine_stage`: wire projection `state`; `get_story` `state_label`
      + `needs_gates` Dev-check; `move_story` `current_state_display`;
      `dev_in_flight`; the COLUMN model (`board_columns`/`column_header`/
      `group_by_position` → StageId-keyed `BoardColumn`; `BoardScope` filter;
      `build_board_text`; `view.rs::parse_state`). `BoardState` unchanged
      (stage redundant). Falsifier: wire + board render + those tool outputs + e2e
      byte-identical. (May split 1a per-story readers / 1b column model.)
    - **BD1b-2 (writes + outbound → event-carried placement).** Split 2a/2b
      along the outbound vs. store axis (each a vertical slice of the
      placement-on-event migration).
      - **BD1b-2a (outbound → event placement) — DONE.** Add
        `placement: Option<StageId>` to `StateTransitioned` (`#[serde(default)]`
        for outbox back-compat). Populate at every producer: deliver → explicit
        `Some(DeskCheck)`, pickup + all coarse targets → `None`; the three
        dynamic-target producers (`fail`/`reject`/`move_story`) derive via the
        transitional `board::placement_for(&state)` bridge (mirrors the target's
        fine stage while `BoardState` still carries it). `push_state_transitioned`
        composes the outbound label from `(new_state, placement)` via
        `mapper::outbound_state_label`, reading the fine grain ONLY from placement
        (the coarse state's own stage no longer decides the middle label). Reconciler
        UNCHANGED (still derives placement from `story.state`, which still carries
        the stage) → placement and the stage agree by construction, so outbound is
        verifiable-identical for real inputs. Falsifier: `outbound_label_tests`
        proves placement (not the embedded stage) drives the desk-check label;
        survives the BD1b-3 unit flip.
      - **BD1b-2b (store → event placement) — DONE.** `BoardPlacementReconciler`
        rewritten to two coarse-only concerns: (1) an EVENT edge — on
        `StateTransitioned{placement}`, Place on `Some` / Unplace on `None`
        (`placement_edge`, reads the event's placement, NOT `story.state`); (2) a
        level SAFETY-NET — `unplace_departed` unplaces any placement whose story
        vanished or left the middle (`matches!(state, InProgress(_))`, a coarse
        variant check that survives the unit flip). The derive-from-`InProgress(stage)`
        place-half is RETIRED. Ingest (both `ingest_once_since` bulk + `refresh_story`
        single, via `seed_and_place`) dispatches `board::placements_for_ingest`
        through `action_tx`: desk-check → Place, Dev-in-middle → Unplace (clears a
        stale desk-check on re-ingest), coarse → skip. `placements_for_ingest` reads
        the decoded `story.state` stage TRANSITIONALLY (the BD1b-3 decode-emit-fine
        split replaces it — deferred, since the decoded Story still carries the stage
        in 2b). The `fine_stage` own-stage fallback is the parallel-change overlap:
        an unplaced desk-check story STILL renders desk-check via the fallback, so
        wire/render is byte-identical whether or not the placement lands — 2b
        pre-populates so BD1b-3 can drop the fallback mechanically. `BoardState`
        unchanged. Falsifier: `board_placement_tests` (edge reads placement not
        state; safety-net is coarse-only) + `placement_tests` (ingest placements) +
        full suite 2381 + e2e byte-identical.
    - **BD1b-3 (the flip).** `InProgress(StageId)` → unit. Fix ~140
      constructors/matches. `as_str`/`github_label` → coarse for InProgress;
      `parse` stops EMITTING desk-check but still TOLERATES `desk-check`/`desk_check`
      → coarse InProgress (migration-tolerant decode for legacy GitHub labels +
      persisted outbox events). `state_from_labels` → coarse InProgress (the
      desk-check→placement split now fully in the BD1b-2 ingest dispatch). Drop the
      own-stage fallback in `fine_stage` → `placement.unwrap_or(Dev entry default)`;
      retire `story_board_position`/`story_position` composing if it goes to
      identity. Relocate `enums_tests` desk-check round-trips → board/placement
      tests. Falsifier: wire + render + outbound + e2e byte-identical.
- **BD2 — placement persistence.** Refinery migration + repo port
  (storage-ports pattern), write-behind listener, hydrate on boot.
  **Loss-window call (sudden-death rule): decide explicitly whether losing
  the last placement move on crash is acceptable** (default recommendation:
  yes — placement is cheap to re-derive by working; document the choice).
- **BD3 — ⚠ the desk-check de-leak (owner-approved shared-edge change).**
  Encode: every `InProgress(_)` → the `"in progress"` label (the codec speaks
  SECTION, never sub-section). Decode: `"in progress"` → In Progress +
  placement from local storage (entry default). Migration decode: legacy
  `"desk-check"` label → In Progress + desk-check placement seeded locally;
  label rewritten to `"in progress"` on next push (lazy, not a sweep).
  Cross-instance desk-check visibility ends BY DESIGN.

## A2 — the verdict collapse, two-grain-native [deep]

`Verdict { Succeed, NeedsWork(target) }` fired by a state's judge; one
`advance` derives the next position. Designed against the grains:
- Bookend crossings (pickup, enter-acceptance, accept, reject-to-ready) =
  STORY EVENTS — shared plane, gated, evidenced, gate-commented.
- Within-middle moves (deliver, desk-check pass/fail routing inside the
  middle) = BOARD ACTIONS — local. (`pass_desk_check` crosses the acceptance
  bookend, so its ceremony/evidence is unaffected.)
- **Intents are atomic**: one intent = ONE action applied atomically by the
  reducer — never the current 3-event dispatch sequences.
- `workflow/verdicts.rs` (+`verdict_persistence.rs`) relocates INTO
  `product_management` — the evidence verdict UNIFIES with the transition
  verdict (never a rival type); dissolves the verdicts.json third-substrate
  anomaly; closes the last workflow↔pm cycle arm; the workflow→pm fitness
  lock lands after this final cut.
- RISK: `GateComment` is persisted (issue comments, fetched back) — needs a
  back-compat decode as `gate` becomes `verdict + target`. 2–3 sub-slices.
- Absorbed handler cleanups (same files, avoid double-touching): remove
  inline `drain_pending` calls (writes return optimistically; pending-ness =
  a selector over the outbox — add `pending_operations_for` here); fold the
  body/title write-through into the outbox; fix the `close_iteration`
  double-dispatch; **re-express the backend story-terminal `is_closed` reads
  as `position ∈ {accepted, retired}`** (ex-C2; the field itself and the wire
  stay untouched until SC).
- Sub-slice 0 (structural prep): make the `EventMeta` envelope structural —
  `StoryEvent { meta, id, kind }` — killing the 26-arm `meta()`/`story_id()`
  double-match before A2 edits the enum. Serde-compat verified against
  persisted shapes.

## A3 — delete the `Gate` enum

Once handlers speak verdicts: `gate_as_str`
(`bridge/tools/view_story_timeline.rs`), `gate_to_id`
(`acceptance/helpers.rs`), `reject_gate_label` (`bridge/tools/rework_context.rs`)
collapse. Enum at `story/enums.rs:147`.

## M — metrics re-sourcing (⚠ metric redefinition, owner-approved)

Zero message parsing. Anti-confabulation guarantee (every number traces to an
observable signal) rides the histories' timestamps/actors.
- **M1** — write-time transition capture: the store records transitions as
  they happen (shared history from verdict records; local history from board
  placement moves).
- **M2** — GitHub backfill on cold start (gate/verdict comments are the
  durable source; thin metrics until backfill completes — accepted).
- **M3** — the two selector families: **velocity = Tracker semantics**: count
  of accepted business-value stories per iteration, each worth 1 (points mean
  nothing; #251 features-only filter stays), rolling average — shared-plane,
  identical on every instance. **Flow (DORA-shaped)**: lead time
  pickup→accept + acceptance rejection rate (shared); sub-section timings +
  desk-check rework (local, per-instance BY DESIGN).
- **M4** — delete the chat-parse velocity (`view/velocity/`), re-point
  `view_velocity` to the selectors.

## C1 — shed `ready_order` [after firsthand consumer proof]

Remove field + codec (`product_management/label_codec.rs`,
`integrations/github/gh_label_codec/mod.rs`) + wire emission
(`view/board/story.rs`) + ingest decode. Refactor constraint: the wire
emission and label may only be removed after FIRSTHAND proof every consumer
(renderer + agents) is dead.

## D — finalize order-on-lists

Order lives purely on `Iteration.stories` index; remove remnants; the
roadmap tree is the single order source.

## SC — selector consolidation sweep + locks

- Stories get real named selectors (today: none — raw
  `state.stories.values()` closures in `sync/slice_readers.rs`); migrate the
  remaining `crate::view` derivations into domain selectors; surfaces
  (bridge render fns, REST, WS snapshot) consume selectors and only format.
- **Ex-C2 (absorbed):** shed `is_closed` off the Story struct + `gh_state`
  off the wire as the `columnFor` mirror retires; re-home the
  closed-iteration concept to `Iteration.complete` (renderer reads the
  container, not the marker's `gh_state`). Renderer-coordinated; any visible
  change to closed-iteration rendering is designer-owned.
- Fix the Rust-internal drifts en route: the `!complete` current-iteration
  guard (`transport/rest/board.rs`); state-display-labels declared
  per-surface presentation.
- Fitness locks for the unowned seams: bridge→*, sync→store,
  integrations→store, view scope, "surfaces don't touch raw slice state."

## WR — write-path remainder

- Drain trigger action→state (reconciler over "outbox non-empty";
  `SyncAction` command-actions retire).
- Conflict machinery retirement (LWW = ordering, not conflict; verify
  `ConflictDetected/Resolved` consumers firsthand before deletion; GitHub
  edit history is the prose recovery ledger).
- Reactions dual-write → one source of truth (handler dispatches; a listener
  persists).
- Shadow-field sheds, one slice each: `Epic.state` (its own derivation
  ignores it), `story_type` (derivable from title), `iteration_slot`
  (mirrors membership; coordinates with C1/D).

## SR — session renames [last; mechanical]

"Session" is a dissolved concept; rename each referent for what it IS:
renderer synced slice `session` → `state`; backend `Session` struct → derive
from its responsibilities (the delivery hub); `session_dir` → `data_dir`
(+ `BPMUX_SESSIONS_BASE`, traveling with its test-harness consumers).
Internal renames first; any wire-visible "session" key needs the wire check.

## Standing rules while executing (from architecture-rules.md)

Env-config ≠ instance strategy (`workflow.toml` never moves to env). Sudden
death is a supported input — every new write-behind listener chooses its
loss-window explicitly. State lives only in the store; db + GitHub are
listener-written durable projections, read only at hydrate/backfill. Events
write; state triggers. One judge per state is load-bearing. The middle is
silent externally.
