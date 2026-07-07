# R-3: collapse Icebox + Ready into NotStarted

Status: **dev design against the settled model** (see `story-lifecycle-type.md`,
"The settled model", Ryan 2026-07-06). This doc turns the model's R-3 bullet into
the exact derivation, verb contract, and slice sequence — including the one new
agent-facing contract edge, disclosed here before any code.

## The settled model applied

After R-3, a story stores `work ∈ {NotStarted, InProgress, Acceptance, Accepted,
Retired}`. Icebox-ness and ready-ness are no longer stored anywhere; they derive:

```
ready(story)   = work == NotStarted  ∧  member of the CURRENT iteration
icebox(story)  = work == NotStarted  ∧  not a member of the current iteration
backlog bucket = Icebox (no iteration) | ReadyForDevelopment (future iteration)
               | Current (current iteration) | Concluded (Accepted/Retired)
```

The wire stays byte-shaped: `/api/board` and the state-delta still carry
`"icebox"` / `"ready"` as story states and column ids — they are now *derived*
labels of the projection, so the renderer does not change in R-3. GitHub keeps
receiving the `ready` label outbound (derived the same way) so the human-readable
mirror is preserved; inbound, membership is the truth and the label is not a
decode input for scheduling (see "Divergences" below).

Membership substrate (surveyed 2026-07-07): `Iteration.stories: Vec<StoryId>`
stored in the iteration issue body (`IterationRelationships`), `is_current` via
the `current-iteration` label, single-membership invariant enforced by
`iteration_members::rewrite_members`. This axis already exists and is already
half-read as the truth: `owner_park::ready_queue_is_empty` is
`state == Ready ∧ in_current_iteration`, `list_ready` orders by
`current.stories`, and `backlog_column`'s `Current` bucket requires current
membership. R-3 finishes what these readers started.

## The verb contract (the re-expression)

The state axis dies, so the scheduling verbs must write the membership axis:

- **`move_story → ready`** = *join the current iteration* (at the given position,
  or at the end when no position is given). Today the bare move writes only the
  state fact and joins the iteration only when a position is passed AND a current
  iteration exists — the drift pair this refactor removes.
- **`move_story → ready` with NO current iteration designated** = **loud error**:
  "no current iteration designated — create and designate one, then schedule."
  There is nothing coherent for ready to mean relative to no iteration (the
  buckets are iteration-relative by the settled model). ★ This is the one NEW
  agent-facing contract edge R-3 introduces. Real team usage always has a
  current iteration (iteration-ceremony canon); the affected walks are test
  shortcuts and misuse, and the error tells the caller exactly what to do.
- **`move_story → icebox`** = *leave the iteration*. Today the icebox move does
  NOT remove membership, which is how the model's named illegal state
  ("Icebox in an iteration") gets created. The design doc names removing that
  state as the point of the collapse.
- **`reject` (acceptance → ready)** keeps working unchanged on canonical walks:
  the story is already a member, so its derived state is ready. On an
  iteration-less server a rejected story derives icebox — divergence
  normalization, see below.
- **Pickup gate** unchanged: `check_target_state_valid` stays lenient (it only
  rejects `current == target`), exactly as today.

## Divergences normalized (the R-2 pattern: one truth, reconcile at ingest)

States reachable today that the collapse makes unrepresentable, and what each
becomes:

1. **Ready but in no iteration** (bare move-to-ready on an iteration-less
   server; GH `ready` label added externally without membership) → derives
   icebox. The outbound label sweep then strips the stale `ready` label —
   self-healing, same shape as R-2.3's closed→terminal forcing at ingest.
2. **Ready but member of a FUTURE iteration** → board-wire derives icebox
   (it is not current work); backlog bucket is ReadyForDevelopment — which is
   what that story always meant. Today it incorrectly renders in the current
   READY column.
3. **Icebox but member of an iteration** → derives ready (current) or
   ReadyForDevelopment bucket (future). Going forward unreachable: the icebox
   move removes membership.

## Slice sequence (expand/contract, each gated + CI-green)

- **R-3.0 — test-shape migration (pure test change, green before AND after the
  flip).** Add a harness helper (`schedule_into_current_iteration(pm, id)`,
  modeled on the journey e2e's scheduling leg) and migrate every e2e/unit walk
  that moves to ready into real iteration scaffolding. No production change.
- **R-3.1 — verb contract flip.** move-ready always joins the current iteration
  (error when none); move-icebox leaves the iteration. State writes remain this
  slice, so the axes agree on every reachable state from here on. The journey
  e2e is the drift-guard.
- **R-3.2 — read derivation.** One shared predicate module; `wire_state`,
  `get_story`, board columns, `backlog_column`, `list_ready`,
  `iteration_close/blockers`, `owner_park` all read ready/icebox from
  (work, membership). Behavior-identical because R-3.1 made the axes agree.
  Decode: the `ready` label stops resolving scheduling (membership decodes from
  the iteration body as today); label-absence stops meaning icebox.
- **R-3.3 — collapse the storage.** `BoardState::{Icebox, Ready}` → `NotStarted`;
  `parse` keeps accepting the legacy `"icebox"`/`"ready"` strings (persisted
  events + live labels exist); producers re-expressed (create → NotStarted,
  reject → NotStarted); outbound GH label derives. The workflow config keeps its
  `icebox`/`ready` state ids — config vocabulary maps onto the projection, the
  same way custom middle-stage ids already map onto InProgress.
- **R-3.4 — re-express the pins.** `board_column_parity_e2e` and
  `state_field_rederive` re-pinned over the derived wire; incident-class tests
  seeded on the dead divergences re-expressed on reachable states.

## Blast radius (survey 2026-07-07)

~50 production sites / ~35 files read, write, or wire the distinction; ~55 src
test files + ~30 e2e touch it. The five most entangled: `story/enums.rs` (the
vocabulary funnel), `move_story/mod.rs` (state+membership written together),
`mapper/state_labels.rs` (label-absence = icebox default), `backlog.rs` (the
derivation R-3 pivots on), and the board wire chain
(`arrangement.rs`/`board/mod.rs`/`rest/board.rs` → renderer `types.ts`).
