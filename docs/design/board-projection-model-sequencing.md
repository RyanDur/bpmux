# Sequencing plan — board-projection-model remodel (#69)

Companion to `board-projection-model.md` (the model). This is the execution plan:
the gated slices that carry today's code to that model. Grounded in two audits
(verdict/transition machinery; state/projection/persistence surface), 2026-07-03.

**Guardrails per slice:** behavior-preserving OR an intended change with tests updated
to the new behavior; each gated (nextest + vitest + e2e) → commit → push → verify
ls-remote. Every slice lands green against the bundled 2-stage workflow
(in-progress → desk-check).

## Order at a glance

`A1` (safe, invisible) → `B1` → `B2` (the visible board/backlog change) → `C2` →
`A2`/`A3` (the deep verdict-first-class + Gate deletion) → `C1` → `D`. Rationale:
delete dead weight first (A1); establish the projection spine before the write-path
surgery (B before deep A, per the B-first call); give `retired`/`accepted` homes
(B2's backlog) before shedding `is_closed` (C2); do the persistence-touching verdict
collapse (A2/A3) last.

## Block A — collapse the transition machinery to verdicts (write path)

- **A1 — delete the ordinal apparatus. [SAFE, behavior-preserving.]**
  Delete `state_ordinal` / `is_backward` / `most_recent_backward_transition` /
  `replay_transitions` / `StateTransition` (`product_management/story/transition.rs`).
  Re-express the only consumer, `get_story.rs`: `workflow_state` = `state_label(story.state)`
  (the replayed value always equaled the stored state); the rework rejection-reason =
  `rework_context::latest_reject(gates).payload` (already the NeedsWork gates,
  `Rejected | FailDeskCheck`). Gate-protected by `get_story_tests`.
  Sites: `transition.rs:14-68`, `get_story.rs:5-6,27-47,114-121`, `story/mod.rs:14-16`.

- **A2 — the Verdict as the transition primitive. [deep; persistence-touching.]**
  `Verdict { Succeed, NeedsWork(target) }` fired by a state's judge; a single
  `advance(position, verdict, workflow)` derives the next position (Succeed → next in
  config order; at acceptance → accepted; NeedsWork → configured fail-target or the
  agent's chosen target). Re-cast the 6 handlers: pickup = board entry (non-verdict),
  deliver/pass-desk-check/accept = Succeed, fail-desk-check/reject = NeedsWork. Fold
  today's split (evidence-verdict posted then consumed by a gate action) into the
  verdict firing directly; the role / evidence-window / gate_class / CI checks survive
  as verdict preconditions. RISK: `GateComment` is persisted (issue comments + fetched
  back) — needs a back-compat decode as `gate` becomes `verdict + target`. Likely 2–3
  sub-slices. Sites: `tools_stories/transition.rs`, `acceptance/{reject,accept,desk_check/*}.rs`,
  `story/events.rs:88-93,255-264`, `integrations/github/mutation.rs:27-32`.

- **A3 — delete the `Gate` enum.** Once handlers speak verdicts:
  `gate_as_str` (`view_story_timeline.rs:6-16`), `gate_to_id` (`acceptance/helpers.rs:32-42`),
  `reject_gate_label` (`rework_context.rs:14-19`) collapse. Enum at `enums.rs:146-155`.

## Block B — projections (read path)

- **B1 — board = group-by-state `Map<State,[StoryId]>`. [behavior-preserving.]**
  Dissolve `ColumnBucket` (`arrangement.rs`); render the same columns from the map.
  Consumers: `board_render.rs`, `view_backlog.rs`, `view.rs:parse_bucket`, the TS
  `columnFor` (`board/types.ts:22-43`), the `types.test.ts` "#514 columnFor" parity.

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
  `get_story`, wire `gh_state`).

- **C1 — shed `ready_order`. [low-risk.]** Already bypassed by membership-order sort
  (`ready_order_label.rs` sorts by iteration index, not the field). Remove field +
  codec (`label_codec.rs`, `gh_label_codec/mod.rs`) + wire emission (`view_model/story.rs:58`)
  + ingest decode; verify the renderer orders by list index, not the `ready-order.N` label.

## Block D — finalize order-on-lists

Confirm order lives purely on `Iteration.stories` index; remove any `ready_order`
remnants; the roadmap tree (outcome/iteration priority order) is the single order source.

## Executes autonomously overnight vs awaits review

- **Overnight (safe, invisible, gate-protected):** **A1**.
- **Awaits Ryan's look at this corrected plan** (owner-visible, persistence-touching,
  or net-new derivation): A2, A3, B1, B2, C1, C2, D. These are sequenced and grounded
  above; on "keep going" they run in the order at the top.
