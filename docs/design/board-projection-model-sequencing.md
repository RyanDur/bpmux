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

`T0` ✅ → `RF` ✅ → `BD1` ✅ (board domain; two grains: coarse position + fine
`BoardPlacements`) → **`A2`** (verdict-MOVE collapse, two-grain-native; A2-1 ✅
A2-2 ✅, **NEXT = A2-2c** then A2-3 [+ optimistic writes] → A2-4 → A2-5 → A2-6 →
A2-0) → **Persistence arc** (`PP` placement-persist + `VP` verdict-into-store,
per-domain file-adapter ports + `BD3` de-leak riding the placement port —
CONVERGED Ryan 2026-07-05: finish A2 first, then per-domain ports; NOT a DB arc,
just port+file-adapter; the DB is a later adapter-only swap) → `A3` (delete
`Gate`) → `M1–M4` (metrics) → `C1` (shed `ready_order`) → `D` (order-on-lists) →
`SC` (selector sweep + locks) → `WR` (write-path remainder) → `SR` (renames).

**★ Persistence arc — converged with Ryan 2026-07-05 (supersedes the earlier
"park BD2/BD3 behind a DB arc" deferral, which was an over-escalation).** Both
verdicts and board placement are (or become) STATE IN THE STORE, each with a
listener → **per-domain repo port** → adapter that persists it. The adapter is a
simple local JSON file NOW; the "DB arc" later swaps that adapter for a
SQLite/refinery adapter and **touches only the adapter** — store slice, listener,
port interface, and every selector stay untouched (the whole payoff of the
pattern; consistent with the five existing repo ports OutboxRepo/UserRepo/…).
- `PP` — board placement persistence: `PlacementRepo` port (board slice) + file
  adapter + write-behind listener + hydrate-on-boot. `BoardPlacements` is already
  store state (BD1) → pure addition. Loss-window: flush-before-ack (silent loss of
  desk-check status on restart is a UX regression).
- `VP` — verdict evidence into the store: TODAY `verdicts.json` is read/written
  DIRECTLY at gate-time (submit_verdict writes; accept gate reads), bypassing the
  store — that gate-time-file-read is the real anomaly. Promote to a pm store slice
  (submit dispatches; gate reads a SELECTOR) + `VerdictRepo` port + file adapter +
  listener + hydrate. Removes the anomaly (better than leaving the raw file).
- `BD3` de-leak (⚠ owner-approved shared edge) rides `PP`: the local placement file
  is the recovery path that replaces the GitHub `desk-check` label. Encode: middle →
  `"in progress"` only. Decode: tolerate legacy `"desk-check"` → In Progress + local
  desk-check placement, rewrite lazily.

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
    - **BD1b-3 (the flip).** `InProgress(StageId)` → unit is a variant-ARITY change
      (atomic — all sites break in one compile-unit). Decomposed into 3 pre-flip
      de-risk slices (each CI-green while the stage still exists, shrinking the flip)
      + the flip.
      - **BD1b-3 Slice A (decode-emit-fine) — DONE.** `state_from_labels` refactored
        into `resolve_state_and_placement(labels, is_closed) -> (BoardState,
        Option<StageId>)` — one precedence pass emits BOTH grains; `state_from_labels`
        + new `placement_from_labels` are `.0`/`.1` wrappers (callers untouched). The
        desk-check arm keys off `Stage::DeskCheck.id().as_str()` (flip-safe, no
        `BoardState` construction). `decode_story_with_placement(v) -> (Story,
        Option<StageId>)` surfaces the label-derived fine stage; `decode_story_metadata`
        delegates. New `Ingested.placements: Vec<(StoryId, StageId)>` carries it —
        populated by BOTH adapters from LABELS (github classify via
        `decode_story_with_placement`; in-memory via `placement_from_labels`), consumed
        by `seed_and_place`. `placements_for_ingest(items, desk_check)` now reads only
        the COARSE `InProgress(_)` from story.state (survives the flip) + the passed
        desk-check signal. Bulk ingest is fully decoupled from story.state's stage;
        the single `refresh_story` path still derives via `placement_for(&story.state)`
        (transitional — Slice D carries the fine stage on the fetch). Falsifier:
        `placement_tests` (the signal drives placement, not the embedded stage). Full
        suite 2381 + e2e byte-identical.
      - **BD1b-3 Slice B (label maps stage→label directly).** `outbound_state_label`
        (story_decode.rs:183), `coalesce.rs:100-101`, and the `resolve_state_and_placement`
        dev arm (`InProgress(Dev).github_label()`) map `Stage`→label string directly
        (DeskCheck→"desk-check", Dev→"in progress") instead of via `InProgress(stage).github_label()`.
      - **BD1b-3 Slice C (column model StageId-keyed).** `board_columns` (arrangement.rs:8),
        `column_header` (:41 binds stage), `parse_state` (view.rs:85). Key columns off
        StageId, not `InProgress(stage)`. NOTE `group_by_position` buckets by whole
        BoardState → post-flip all InProgress collapse to one bucket; verify.
      - **BD1b-3 Slice D (the flip) — DONE.** `InProgress(StageId)` → unit. `as_str`/`github_label`
        → coarse for InProgress; `parse` tolerates `desk-check`/`desk_check`/`in-progress`/
        `in_progress` → unit InProgress (migration-tolerant for legacy labels + persisted outbox).
        `resolve_state_and_placement` `.0` desk-check arm → coarse InProgress (`.1` unchanged, so
        decode still emits the fine stage). `fine_stage` fallback → `placement.unwrap_or(Dev)`.
        Retired `placement_for` (→ `board::placement_from_target` for move_story, deviation-from-entry).
        `fetch_story_metadata` now returns `(Story, Option<StageId>)` so refresh_story places from the
        fetch. ~17 prod + ~120 test constructions dropped the payload; coarse `InProgress(_)`→`InProgress`
        matches; enums_tests desk-check round-trips retired (desk-check is a placement concern, no longer
        a BoardState wire vocab). **Non-obvious consequences handled:** (1) the workflow GATE
        `check_target_state_valid` for fail-desk-check needs the FINE state — `fail.rs` composes
        `board::state_label(&state, r.story_fine_stage(story))` instead of coarse `as_str`; (2) the
        in-memory adapter's `typed_label_mirror::state_transitioned` must compose the outbound label
        from `(new_state, placement)` via `outbound_state_label` (mirror of the github push); (3) the
        seeded test harness (`bind_seeded_refresh`) never wired the Board-action path — added
        `board_dispatch` (a `RefreshDriver` bind mirroring `story_dispatch`) so ingest placements reach
        the seeded store and `story_fine_stage` sees desk-check. Falsifier: full suite 2381 + vitest 766
        + e2e all green; wire/render/outbound byte-identical. TS renderer untouched.
- **BD2 — placement persistence, as a SIMPLE LOCAL FILE (NOT a DB migration).**
  **★ CORRECTED (Ryan, 2026-07-05): the earlier "park BD2 behind the DB/storage-
  ports arc" decision was an over-escalation and is REVERSED.** A `StoryId →
  StageId` map is instance-local state that belongs in a simple local file —
  exactly the `verdicts.json` precedent — not a refinery migration behind a
  per-feature repo port. So BD2 has NO DB-arc dependency and is doable now: a
  board-slice listener writes `BoardPlacements` to a small JSON file on change;
  hydrate reads it at boot. The one improvement over `verdicts.json`: keep it
  store-integrated (listener-written, boot-read only) so it is a proper
  hydrate-only projection, not a gate-time side-channel. **Loss-window call
  (sudden-death rule): write synchronously enough that a crash does not silently
  lose the last placement move** (placement is cheap to re-derive, but silent
  loss on restart is a UX regression — prefer flush-before-ack like the outbox;
  finalize in implementation). Because BD2 is now cheap and immediate, **it no
  longer gates BD3** — the two land together.
- **BD3 — ⚠ the desk-check de-leak (owner-approved shared-edge change; UNBLOCKED
  by simple-file BD2).** Encode: every `InProgress(_)` → the `"in progress"`
  label (the codec speaks SECTION, never sub-section). Decode: `"in progress"` →
  In Progress + placement from the local file (entry default). Migration decode:
  legacy `"desk-check"` label → In Progress + desk-check placement seeded
  locally; label rewritten to `"in progress"` on next push (lazy, not a sweep).
  Cross-instance desk-check visibility ends BY DESIGN. Lands with BD2 (simple
  file is the recovery path that replaces the GitHub `"desk-check"` label).

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
- `workflow/verdicts.rs` (+`verdict_persistence.rs`) relocated INTO
  `product_management` (A2-1 ✅) — closing the last workflow↔pm cycle arm; the
  workflow→pm fitness lock landed. **The evidence verdict STAYS in `verdicts.json`,
  off the story (CORRECTED, Ryan 2026-07-05): there is no "dissolve into the store"
  requirement — that was an invented goal. `verdicts.json` is instance-local
  evidence, fine as a simple file.** The move-`Verdict` (board slice, A2-2) is a
  SEPARATE concept (the Gate-replacement), not a merge of the evidence struct.
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

### A2 sub-decomposition (designed up-front 2026-07-05, executor queue)

Substrate mapped firsthand this session (three Explore sweeps: story-event
envelope; verdict+gate+GateComment machinery; transition handlers). Anchors
below are from that map.

**★ SCOPE CORRECTED (Ryan, 2026-07-05) — I had over-escalated local-persistence
needs into fake DB-arc deferrals; reversed:**

- **Optimistic writes are IN A2-3, not deferred.** Writes returning optimistically
  (per `architecture-rules.md`) was always the intended design; today's blocking
  inline `drain_pending` is the *accident*, not the spec (existing-code-is-not-the-
  spec). A2-3 removes inline `drain_pending`, folds the body/title write-through
  into the outbox, and adds the `pending_operations_for` selector. The async push
  already has a home (the polling drain, `bridge/polling/drain.rs`) so removing the
  inline drain does not strand the GitHub update — verify that path covers every
  verb. Rewrite the tests that assert synchronous propagation (e.g.
  `..._drains_synchronously_so_gh_side_propagates_before_return`) — they pin the
  accident.
- **`verdicts.json` STAYS, off the story — no dissolution, no DB arc.** The
  "dissolve the third substrate into the store" requirement was invented; keeping
  verdict evidence in a simple local file is fine (it is instance-local evidence).
  A2 already keeps it off the story: the move-`Verdict` (board slice) is a SEPARATE
  concept from the evidence `product_management::verdict::Verdict`, not a merge.
  The two coexist as two honest concepts (board move vs evidence-grounding).
- **The desk-check de-leak (middle goes silent to GitHub) is UNBLOCKED — it never
  needed the DB arc.** Placement persists as a SIMPLE LOCAL FILE (the `verdicts.json`
  precedent), so BD2/BD3 land together, downgraded from "refinery migration" to
  "file + de-leak" (see the corrected BD2/BD3 entries above). Until BD3 lands, A2
  keeps the middle's GitHub output byte-identical (the `Delivered` comment + the
  placement-sourced label still emit); the board-action reframing inside A2 is
  internal only. BD2/BD3 can be sequenced right after the A2 verdict slices — no
  arc dependency.

Executor order (A2-0 reordered to the TAIL, see above):
A2-1✅ → A2-2 → A2-3 → A2-4 → A2-5 → A2-6 → A2-0. Each gated → commit → push
(keepalive) → verify `ls-remote` → CI-green gates the next; refactor falsifier =
zero edits to existing test assertions.

- **A2-1 — relocate the evidence-verdict into `product_management`; close the
  last workflow↔pm cycle arm.** Move `workflow/verdicts.rs` (`Verdict`,
  `VerdictsStore`, `AddVerdictOutcome`, `verdicts_path`/`load`/`save`/
  `add_or_replace_verdict`/`verdicts_for_story`) → `product_management/verdict.rs`;
  the test module `workflow/verdict_persistence.rs` → `product_management/
  verdict_tests.rs`. Re-point the ONLY two prod importers — `bridge/tools/
  submit_verdict.rs` (writer) + `bridge/tools_stories/acceptance/helpers.rs`
  (reader `recent_evidence_for_story`) — plus the three test importers. `verdicts.json`
  persistence UNCHANGED (same file, same fns, just re-homed). VERIFY firsthand that
  `workflow` production imports NO other `product_management` symbol, then land the
  proven-to-falsify `workflow` ↛ `product_management` fitness lock. Pure relocation:
  falsifier = import-path edits only, zero assertion changes.
- **A2-0 — `EventMeta` envelope structural.** `StoryEvent { meta, id, kind:
  StoryEventKind }` (all 26 variants carry `meta`+`id` uniformly today —
  `events.rs:11-233`). Collapse the two 26-arm or-pattern accessors
  (`meta()`/`story_id()`) to field access; add `StoryEvent::new(meta, id, kind)`
  to absorb construction churn (~404 refs across ~70 files, ~half tests; five
  total-match dispatchers: reducer, conflict classifier, github push, in-memory
  push, +2 accessors). SERDE-COMPAT is the sharp edge: the on-disk `QueuedEvent.event`
  shape changes (externally-tagged-per-variant → enveloped). The JSON-file outbox
  has an untagged legacy arm (`OutboxEntryOnDisk`, `outbox.rs:23-28`) but the SQLite
  outbox (`sqlite_outbox.rs`) has NONE and there is NO event-level round-trip test.
  So: custom `Deserialize` reading BOTH shapes + ADD a legacy-decode round-trip test
  (new coverage). Falsifier: full suite green; existing assertions unchanged.
- **A2-2 — the move-`Verdict` + `advance`, two-grain-native (expand).** Homed in
  the `board` slice (NOT pm): `advance` needs the fine state order AND the
  projection to `(coarse BoardState, Option<placement>)` via
  `board::placement_from_target`; that projection is board's, and `board` already
  depends on `pm::story`, so homing here reuses it with NO new cycle (pm calling
  board would recreate the pm↔board cycle the refactor kills).
  - `enum Verdict { Succeed, NeedsWork(BoardState) }` (the move outcome; the
    Gate-replacement, not the evidence type).
  - `advance(from_fine_id, &Verdict, &WorkflowConfig) -> Option<(BoardState,
    Option<StageId>)>`: Succeed → next fine state in `workflow.state` order,
    projected (`BoardState::parse` folds coarse; `placement_from_target` yields the
    sub-section — desk-check → `(InProgress, Some(DeskCheck))`, in-progress →
    `(InProgress, None)`); NeedsWork(target) → `(target, None)` (backward lands at a
    coarse entry). Reproduces every current handler target for the bundled config
    (`bundled_default.rs:64-153`) — pin all six in unit tests.
  - **A2-2a** = the type + `advance` + tests, inert (pub, no dead-code, no handler
    wired) — BD1a-1 style foundation.
  - **A2-2b** = rewire the two single-function Succeed handlers — `deliver`
    (`transition.rs`) and `pass-desk-check` (`pass.rs`) — to derive their
    emit-target via `advance(&transition.from, &Verdict::Succeed, workflow)`,
    replacing a genuine hardcode (`InProgress`+`DeskCheck` / `Acceptance`). The
    `Gate` value stays hardcoded (its derivation from `(fine_from, verdict)` and the
    `Gate` enum deletion are A3). Falsifier held: full nextest green, zero assertion
    edits. The other verdict-handlers get their `Verdict` expression in A2-3, where
    they restructure anyway: `accept` is split (`handle_accept_story` +
    `accept_via_canonical_store`, with a REST twin in `board_acceptance.rs`) so it
    needs target-threading; `fail`/`reject` already derive their target from
    `transition.to` (they don't hardcode), so wrapping them in `NeedsWork` is folded
    into the atomic-intent restructure, not double-touched here. Backward = NeedsWork
    read off the move (composes with A1's `most_recent_needs_work`,
    `story/rework.rs`).
- **A2-3 — atomic intent (per bookend, may split per handler).** Collapse each
  handler's multi-event `mutate_sync` sequence into ONE atomic verdict action the
  reducer applies atomically, producing the SAME outbox ops + final state. Today:
  accept = `GateCommentAppended`+`StateTransitioned`+`StoryClosed` (3 `mutate_sync`,
  `accept.rs:120-148`); pass_desk_check = 4 events (`pass.rs:89-114`); pickup/deliver/
  reject/fail = 2-3 each. **A2-3 also lands optimistic writes (corrected scope):
  remove inline `drain_pending`, fold the body/title write-through into the outbox,
  add `pending_operations_for`; the polling drain (`bridge/polling/drain.rs`) is the
  async push path — verify it covers every verb, then rewrite the synchronous-drain
  assertions.** Verify no listener depends on an intermediate half-applied state (the
  `BoardPlacementReconciler` reacts to `StateTransitioned` — the atomic action must
  still surface that effect). May split: atomicity (structural) vs optimistic-writes
  (behavioral, updates tests) as separate slices if cleaner.
  - **★ SUB-DECOMPOSITION (2026-07-05, from a firsthand kernel/drain/listener map).**
    The map surfaced the load-bearing fact: **optimism already exists structurally** —
    every `mutate` (`sync/drain/mod.rs:28`) fires `StartUpdateGithub` → the async
    `GithubDrain` listener (`store/sync/drain.rs`) pushes, and the 5s polling loop
    (`bridge/polling/drain.rs`) drains the SAME outbox via the SAME `drain_once()`. So
    the inline `drain_pending().await` in each handler is a pure SYNCHRONY device
    (push-before-return + surface push-failure to the caller). Three orthogonal slices:
    - **A2-3a — optimistic writes [Ryan's must-not-defer item; do FIRST].** Remove the
      inline `drain_pending` from the six lifecycle-transition handlers (pickup/deliver/
      pass/accept/reject/fail) so writes return immediately; the async drain already
      covers them (no coverage gap — identical `drain_once` over the identical outbox).
      Rewrite the synchronous-drain assertions (`integration_drain_tests.rs`) to DRIVE
      the async drain then assert — incl. `deliver_story_propagates_drain_failure`,
      which flips from "returns Err" to "returns Ok, failure surfaces via async
      `MutationRolledBack`" (THE intended optimistic-failure behavior, not a regression).
      TDD discriminator: assert `pushed_events` EMPTY at handler-return, populated only
      after driving the drain (fails against inline-drain → red; green after removal).
      `pending_operations_for` selector ONLY if a per-story pending consumer exists
      (outside-in — today only a global `SyncStatus.pending_events` COUNT is surfaced;
      do NOT build a per-story selector ahead of a consumer). VERIFY first: no
      production consumer round-trips through GitHub synchronously post-write (map says
      reads are store-optimistic — confirm).
    - **A2-3b — atomic intent [structural, deep].** Add a batch-apply kernel primitive
      (`mutate_batch(Vec<StoryEvent>)`): one store dispatch (listeners fire ONCE on the
      final state, killing the sequential-`mutate_sync` reconciler race the map flagged)
      + N `QueuedEvent`s enqueued in order. Rewire each handler to build its event Vec
      and dispatch once. `BoardPlacementReconciler` must still see the `StateTransitioned`
      placement edge in the folded state.
    - **A2-3c — body/title into the outbox.** `write_through.rs` `update_body`/
      `update_title` bypass the outbox entirely (synchronous adapter push, no `mutate`).
      Enqueue `BodyUpdated`/`TitleUpdated` (variants already exist) through `mutate`, and
      teach `drain_once`/`classify_conflict` to route them to `adapter.update_body`/
      `update_title` (a DIFFERENT adapter method than `push_event`, `trait_def.rs:55,60`).
- **A2-4 — `GateComment` carries verdict+target (back-compat, 2-3 sub-slices).**
  Reshape `GateComment.gate` → verdict+target internally (`story/mod.rs:38-44`) while
  keeping the GitHub comment-codec markers byte-identical (`comment_codec.rs`
  encode/`classify_gate` decode — incl. the `rejection_reason`+`desk-check-fail`
  disambiguation) and serde `#[serde(default)]` back-compat for persisted snapshots
  + `StoryEvent::GateCommentAppended`/`GateCommentsFetched` payloads. Falsifier:
  comment wire byte-identical; legacy-comment decode unchanged.
- **A2-5 — `is_closed` story-terminal recast (ex-C2).** Re-express the 4 core
  backend reads (`view/board/arrangement.rs:23` `board_position`;
  `product_management/backlog.rs:16`; `transport/rpc_routes/state_delta.rs:46`
  `effective_state_label`; `bridge/move_story/target.rs:84`) + the 6 reopen-guards
  (`transition.rs:40,157`; `pass.rs:78`; `fail.rs:44`; `reject.rs:77`;
  `move_story/mod.rs:61`) as `position ∈ {accepted, retired}`. Field + wire stay
  (shed in SC). Falsifier: behavior-identical.
- **A2-6 — `close_iteration` double-dispatch fix.** The bridge handler
  (`closure.rs:94`) fires its own WS `iteration_close_signal` AND self-HTTP-POSTs to
  the REST path (`iteration_close/close.rs:23`) which independently broadcasts
  `IterationClose`/board-invalidate/stories. Collapse to one in-process dispatch.
  CARE: confirm the duplicate is observably redundant (renderer-idempotent) so the
  fix is behavior-preserving, not behavior-changing; if the second announce carries
  distinct data, reconsider scope.

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
