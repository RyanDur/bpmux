# Cassiopeia label-substrate migration audit-log

**Story:** #1301 — "Existing iterations carry forward as positional markers — Cassiopeia Slice D"
**Date captured:** 2026-05-16
**Epic:** #1189 — Iteration markers + canonical-via-GH substrate

## User-job invariant preserved

> All board substrates retire from local `priority.json` to canonical-
> via-GH labels so cross-instance collaboration sees coherent state.
> Board state is shareable; collapse state stays local.

## Three substrates retired

`priority.json` carried three fields at the start of Cassiopeia. All
three retire as part of #1301 broader scope.

### 1. `icebox_unified` — retired in slice 3 (commit 298dc217)

**Was:** Vec of `IceboxItem::Iteration { number }` + `Story { number }`
entries giving positional ordering for the icebox lane (iteration
containers + loose stories interleaved).

**Replacement:** `iteration-{Theme}.{position}` labels on member
stories (canonical-via-GH). Theme name parsed from iteration-issue
title; position is sparse-numeric u32. Substrate landed under #1300.

**Migration ceremony:** `migrate_icebox_unified_to_iteration_labels`
ran once at boot, emitting paired TagAdded events for each member
story. Idempotent: skipped if any story already carried an
`iteration-*.N` label. Captured 2026-05-15 in
`2026-05-15-cassiopeia-iteration-migration.md`.

**Retirement:** The field, the `IceboxItem` enum, the migration
function, the auto-heal-icebox boot ceremony, and the
`IceboxMigration` state-delta variant on both sides of the wire
were all deleted in slice 3.

### 2. `current_iteration_stories` — retired in slice 4A (commit 910ce928)

**Was:** Vec of issue numbers, written when a story moved to
`Ready | InProgress | DeskCheck | Delivered`. Legacy ready-list
substrate from before the canonical workflow_state landed.

**Replacement:** None — post slice 3 the field was write-only with
no readers. Pure dead substrate. Canonical workflow_state on
each story serves the role this field originally tracked.

**Retirement:** Field deleted; `apply_move_story_priority` lost
its `target_is_in_flight` parameter (now-dead); a test assertion
on the write disappeared along with the writer.

### 3. `columns` — retiring across slices 4B → 4E

**Was:** `HashMap<String, Vec<u64>>` mapping each state name
(`ready`, `icebox`, etc.) to a positional ordering of issue
numbers within that column. Written by `reorder_story` /
`move_story` / `post_reorder_story` whenever the user dragged.

**Replacement:** `ready-order.{position}` labels on each Ready
story (canonical-via-GH). Sparse-numeric u32 position; default
first = 100, default step = 100; midpoint algebra on insert.

Other columns (`icebox`, `in-progress`, etc.) had no remaining
user-orderable behavior post slice 3 — icebox ordering moved to
`iteration-{Theme}.N` labels under #1300; non-ready terminal
states aren't user-drag-reorderable in the renderer
(`Board.tsx:589` enables drag only on `ready` and `icebox`).

**Migration ceremony:**
`migrate_legacy_ready_to_labels_at_path` runs at boot.

  1. Idempotency check: if any Ready story already carries a
     `ready-order.*` label, the migration treats canonical-via-GH
     as the source-of-truth and is a no-op (#1301 Scenario 6).
  2. Load `priority.json`; extract `columns["ready"]`.
  3. For each issue number in legacy order, emit
     `TagAdded(ready-order.{(idx+1)*100})` via `mutate_sync`.
  4. Stories listed in legacy but absent from the canonical Ready
     state are skipped (defensive: stale priority.json entries
     from pre-#1272 races).

**Wire-up:** `server.rs::run_server_only` calls
`migrate_legacy_ready_to_labels` after `auto_heal_ready_column_at_boot`
(so the canonical Ready set is clean before label emission).
Logs migrated count via `session::diag::log_diag`.

**Retirement (slice 4E, pending):** Once migration validates in
production, the `columns` field, `apply_priority_to_board`,
`auto_heal_ready_column_at_boot`, and the legacy POST
`/api/stories/:n/reorder` endpoint all retire. `priority.json`
becomes empty / removable.

## Pre-Cassiopeia iteration-name disambiguation

The 2026-05-15 audit-log captured the Aquila → Aquila-B rule for
duplicate iteration titles. That rule is unchanged by this slice;
the iteration-label substrate is already in place.

## Cross-instance coherence (Scenario 1)

All three retired substrates were local to `~/.bpmux/priority.json`,
so cross-instance disagreement was the default. Post-migration, all
state lives in GH labels and propagates via the canonical mutation
chain → state-delta broadcast → multi-instance reconciliation.

Verified end-to-end during slice 4C.2 via Board.tsx label-emit →
adapter LabelsRefreshed-suppression (slice 4B) → store apply →
broadcast → other instance refetch.

## Slice ordering reference

| Slice | Scope | Commit |
| ----- | ----- | ------ |
| 3 | retire `icebox_unified` field + enum + IceboxMigration state-delta | 298dc217 |
| 4A | retire `current_iteration_stories` (write-only dead substrate) | 910ce928 |
| 4B | `ready-order.N` reader substrate + adapter suppression | 5b78b4ae |
| 4C.1 | server reader prefers labels over legacy `columns["ready"]` | 97d7c05e |
| 4C.2 | renderer Board.tsx drag emits ready-order label mutations | f1215231 |
| 4D | boot migration `columns["ready"]` → `ready-order.N` labels + this doc | (this commit) |
| 4E | retire `columns` field + `apply_priority_to_board` + legacy `/reorder` | pending |

## Notes

The `feedback_inline_comment_gate_no_carveouts` lesson fired during
slice 3's commit and triggered a fixup commit (ff60d565). Pre-commit
gate rejects added inline `//` lines unconditionally; commit body
+ this audit-log carry the WHY context instead.
