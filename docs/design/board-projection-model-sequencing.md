# Sequencing plan — board-projection-model remodel (#69)

Companion to `board-projection-model.md` (the model). This is the execution plan:
the gated slices that carry today's code to that model. Grounded in two audits
(verdict/transition machinery; state/projection/persistence surface), 2026-07-03.
Re-grounded against substrate at HEAD `97979a6e` (2026-07-04, after the board
evaporation): A1 was found already landed (`1685ab96`, 2026-07-02); all site paths
below refreshed to their post-evaporation homes.

**Guardrails per slice:** behavior-preserving OR an intended change with tests updated
to the new behavior; each gated (nextest + vitest + e2e) → commit → push → verify
ls-remote. Every slice lands green against the bundled 2-stage workflow
(in-progress → desk-check).

## Order at a glance

`A1` (✅ landed) → `B1` → `B2` (the visible board/backlog change) → `C2` →
`A2`/`A3` (the deep verdict-first-class + Gate deletion) → `C1` → `D`. Rationale:
delete dead weight first (A1); establish the projection spine before the write-path
surgery (B before deep A, per the B-first call); give `retired`/`accepted` homes
(B2's backlog) before shedding `is_closed` (C2); do the persistence-touching verdict
collapse (A2/A3) last.

## Block A — collapse the transition machinery to verdicts (write path)

- **A1 — delete the ordinal apparatus. ✅ LANDED `1685ab96` (2026-07-02), CI-green.**
  Deleted `state_ordinal` / `is_backward` / `most_recent_backward_transition` /
  `replay_transitions` / `StateTransition` (`product_management/story/transition.rs`,
  whole file). Two consumers re-expressed (the audit had found one; the commit found a
  second, wire-facing one): `bridge/tools/get_story.rs` AND the wire
  `StoryView.rework_comment` (now `view/board/story.rs` post-evaporation). Both ride
  `most_recent_needs_work(gates) = gates.last().filter(Rejected | FailDeskCheck)`
  (`product_management/story/rework.rs`) — "the most recent NeedsWork verdict" by
  another name, seeding A2's model. `workflow_state` keeps deriving the backward
  TARGET for a carried-reject story whose issue is github-closed (a 2-arm gate→target
  map that A2's verdicts formalize). Behavior-preserving; 73 affected tests green.

- **A2 — the Verdict as the transition primitive. [deep; persistence-touching.]**
  `Verdict { Succeed, NeedsWork(target) }` fired by a state's judge; a single
  `advance(position, verdict, workflow)` derives the next position (Succeed → next in
  config order; at acceptance → accepted; NeedsWork → configured fail-target or the
  agent's chosen target). Re-cast the 6 handlers: pickup = board entry (non-verdict),
  deliver/pass-desk-check/accept = Succeed, fail-desk-check/reject = NeedsWork. Fold
  today's split (evidence-verdict posted then consumed by a gate action) into the
  verdict firing directly; the role / evidence-window / gate_class / CI checks survive
  as verdict preconditions. This is also where `workflow/verdicts.rs` (+
  `verdict_persistence.rs`) relocates INTO `product_management` — the evidence-verdict
  store UNIFIES with the transition verdict (never a rival type), closing the last
  `workflow↔pm` cycle arm; the `workflow → pm` fitness lock lands here, after that
  final cut. RISK: `GateComment` is persisted (issue comments + fetched back) — needs
  a back-compat decode as `gate` becomes `verdict + target`. Likely 2–3 sub-slices.
  Sites: `bridge/tools_stories/transition.rs`, `bridge/tools_stories/acceptance/
  {reject,accept,desk_check/*}.rs`, `story/events.rs` (`GateCommentAppended` /
  `GateCommentsFetched` / `impl GateComment`), `integrations/github/mutation.rs`,
  `workflow/{verdicts,verdict_persistence}.rs`.

- **A3 — delete the `Gate` enum.** Once handlers speak verdicts:
  `gate_as_str` (`bridge/tools/view_story_timeline.rs:6`), `gate_to_id`
  (`bridge/tools_stories/acceptance/helpers.rs:32`), `reject_gate_label`
  (`bridge/tools/rework_context.rs:14`) collapse. Enum at `story/enums.rs:147`.

## Block B — projections (read path)

- **B1 — board = group-by-state `Map<State,[StoryId]>`. [behavior-preserving.]**
  Dissolve `ColumnBucket` + `column_for` (`view/board/arrangement.rs` — note
  `column_for` also folds `is_closed` into a `Retired` bucket, a C2 entanglement to
  preserve until C2 sheds it); render the same columns from the map. Consumers:
  `bridge/tools_stories/board_render.rs`, `bridge/tools/view_backlog.rs`,
  `bridge/tools_stories/view.rs:parse_bucket`, the TS `columnFor`
  (`renderer/src/board/types.ts`), the `types.test.ts` "#514 columnFor" parity.

- **B2 — narrow the board to `ready…acceptance` + introduce the 4-column backlog.
  [INTENDED CHANGE — owner-visible.]**
  `accepted` / `retired` / `icebox` leave the board. New backlog projection derived
  from (position + iteration membership), reusing existing membership machinery
  (`Iteration.stories`, `is_current`, `current_iteration_id`, `buildMembershipIndex`):
  `accepted`→accepted, current-iteration→current, in-any-iteration→readyForWork,
  else→icebox; `retired`→archived (off both). Update board/backlog tests + renderer.
  This is the big visible slice — hold for a look before landing unattended.

## Block C — shed the derived story fields

- **C2 — shed `is_closed`. [after B2.]** Re-express the ~20 read sites as
  `position ∈ {accepted, retired}`; `is_closed` becomes edge-derived at the adapter
  (github-closed ↔ terminal position). `retired` stays a position; accept/close set
  position and the edge closes the issue. Sites listed in the audit
  (`column_for`, `state_from_labels`, acceptance/reject/desk-check guards, `move_story`,
  `get_story`, wire `gh_state`); full grep spread at re-grounding = 34 files
  (production + tests), so budget the test re-expression alongside the ~20 read sites.

- **C1 — shed `ready_order`. [low-risk.]** Already bypassed by membership-order sort
  (`view/board/ready_order_label.rs` sorts by iteration index, not the field). Remove
  field + codec (`product_management/label_codec.rs`,
  `integrations/github/gh_label_codec/mod.rs`) + wire emission (`view/board/story.rs`)
  + ingest decode (`integrations/github/mapper/story_decode.rs`); verify the renderer
  orders by list index, not the `ready-order.N` label.

## Block D — finalize order-on-lists

Confirm order lives purely on `Iteration.stories` index; remove any `ready_order`
remnants; the roadmap tree (outcome/iteration priority order) is the single order source.

## Executes autonomously overnight vs awaits review

- **Overnight (safe, invisible, gate-protected):** A1 — ✅ landed `1685ab96`; nothing
  overnight-safe remains.
- **Awaits Ryan's look at this plan** (owner-visible, persistence-touching, or
  net-new derivation): B1, B2, C2, A2, A3, C1, D — every remaining slice. Sequenced
  and grounded above; on "keep going" they run in the order at the top, resuming
  at B1.
