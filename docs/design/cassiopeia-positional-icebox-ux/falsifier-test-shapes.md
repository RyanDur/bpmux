# Falsifier test shapes

*Designer — 2026-05-15. Test shapes the user-job falsifiers per Ryan-TDD canon ("failing-test-first → make pass → no-regression confidence"). Dev-lane writes actual code; designer specs the falsifier-shape.*

## Why this doc

The 4 reworked stories (#1297 / #1339 / #1300 / #1301) have body-Scenarios listing observable falsifiers. Some Scenarios are pinnable at the substrate-axis (fireEvent in jsdom; RTL); others require user-axis (real-mouse-drag in browser; Playwright multi-context). This doc lists the failing-tests needed per Scenario + which axis pins them.

## #1297 Slice A — Iteration container

| Scenario | Axis | Test shape | Status |
|---|---|---|---|
| 2: container collapsible + state persists | jsdom + RTL | seed iteration; render; fireEvent toggle; assert open state survives remount via useIterationCollapse | ✓ shipped (iteration-renders-as-collapsible-container.test.tsx) |
| 3: Unscheduled exists for loose stories | jsdom + RTL | seed loose icebox stories; render; assert board-iteration-unscheduled with data-iteration-kind="unscheduled" | ✓ shipped (same file) |
| 1: cross-instance container visible after creation | Playwright multi-context | open 2 BrowserContexts on same backend; instance A creates iteration; assert instance B's icebox shows container within state-delta cadence | PENDING |

## #1339 Slice B — Drag

| Scenario | Axis | Test shape | Status |
|---|---|---|---|
| 1a: drag-within persists across refresh | jsdom + RTL fireEvent | seed members; fireEvent.drop; refetch /api/board; assert dragged story's label = new position; old label removed | ✓ shipped (iteration-drag-within-persists.test.tsx) |
| 1b: drag-within visible-order in DOM matches canonical | jsdom + RTL | sister to 1a; assert post-drop, DOM data-iteration-member attributes match expected order | candidate add to existing test |
| 2: cross-instance order coheres | Playwright multi-context | instance A drags; assert instance B's container updates within state-delta cadence | ✓ shipped (cross-instance-iteration-reorder.spec.ts, 63b66fc7) — `launchTwoElectronsSharedBackend` infra + 15s state-delta convergence assertion |
| 3: cross-container drag retags | jsdom + RTL fireEvent | seed iteration A + B; drag member from A onto B's member; assert dragged story label changes from `iteration-A.N` to `iteration-B.M`; A's body removes the member; B's body adds | ✓ shipped (iteration-drag-across-retags.test.tsx, 3ea5e5d8) |
| 4a: ChosenPicachu hide-on-drag | jsdom + RTL fireEvent | dragStart sets data-dragging="true" on source; CSS `.board-iteration-member[data-dragging='true']` applies opacity 0; dragEnd clears | ✓ shipped (iteration-drag-hide-visual.test.tsx, d98e6712) |
| 4b: ChosenPicachu eager-reorder during drag-over | jsdom + RTL fireEvent | dragOver on a different member sets hoveredNumber; eagerReorder() pure helper moves dragged to hovered's index in render array; DOM data-iteration-member order reflects | ✓ shipped (iteration-drag-eager-reorder.test.tsx, 590ba280) |
| 4c: ChosenPicachu full visual in real browser | Playwright | end-to-end visual capture during real-mouse-drag; verify opacity transition + list reflow + drop-position-confirmed-visually | PENDING (Playwright lane) |
| 5a: closed iteration cannot drag OUT | jsdom + RTL | seed closed iteration with member; assert member has `draggable=false` | covered by Iteration.tsx draggable predicate; not separately RTL-pinned |
| 5b: closed iteration drop-rejection | jsdom + RTL fireEvent | seed open + closed iterations; drag member from open onto closed's body; assert dropEffect=none + data-drag-rejected="true" + no cross-retag | ✓ shipped (closed-iteration-rejects-inbound-drag.test.tsx, 936cf7f7) — CSS shake keyframe + cursor not-allowed |
| 6: outbox coalesce drag-storm | substrate-axis Rust | last-update-wins per (story, iteration-prefix-class); 5 falsifier tests pin: rapid-drag → first-R+last-A only / cross-prefix-class independent / cross-story independent / non-iteration labels untouched / Add-only keeps last-A | ✓ shipped (step 2e, 796f336c) |
| 7: iteration-itself reorder | jsdom + RTL fireEvent | drag iteration's summary-row; assert `iteration-order.N` label updates on iteration-issue | ✓ shipped (iteration-itself-drag-persists.test.tsx, 688d12f1) — note: iteration-loose source (oldOrder=null) deferred; eager-reorder + hide-on-drag at iteration-level deferred per dev commit-note |
| 8: keyboard-reorder a11y parity within iteration container | jsdom + RTL fireEvent | Ctrl+Shift+ArrowUp/Down on focused iteration-member's `.board-iteration-member-button`; assert label mutates to sparse-midpoint position (UP: between prev-1 and prev; DOWN: between next and next+1); closed-iteration no-op; first/last no-op via early-return | ✓ shipped step 2h-1 (keyboard-reorder-within-iteration-persists.test.tsx, 4ff82980) — sister-steps 2h-2/2h-3 pending: announce-text (SR live-region), explicit closed-iteration aria-disabled, first/last announce-text feedback |

## #1300 Slice C — Close iteration

| Scenario | Axis | Test shape | Status |
|---|---|---|---|
| 1: close promotes members user-saw | substrate Rust integration | seed iteration with N labeled members; assert close walks `iteration-{Name}.M` labels; promotes those stories to ready in label-position order | ✓ shipped (close_iteration_tests.rs `close_walks_by_iteration_theme_label_not_by_priority_json_position`, 2b9545d1) |
| 2: closed container stays at position cross-instance with closed-state visual | Playwright multi-context + jsdom | jsdom: assert closed-iteration container renders with data-closed="true" + dim CSS treatment; Playwright: assert cross-instance reflects | ✓ shipped (cross-instance-iteration-close.spec.ts, 6b9f964d) — also fixes silent state-delta gap (post_close_iteration_canonical was missing broadcast_board_invalidate) |
| 3: render-source = close-walk-source invariant | substrate Rust | unit test on the close-iteration helper: feed it the canonical label-set; assert promoted-stories-set matches the visible-order the render derived from the same labels | ✓ partially covered by symmetric `iteration_label.rs` module (4 helpers + 14 unit tests) mirroring renderer's `Iteration.tsx` parse-regex — symmetry pins render-source = walk-source by construction |

## #1301 Slice D — Migration

| Scenario | Axis | Test shape | Status |
|---|---|---|---|
| 1: post-migration state coherent cross-instance | Playwright multi-context | seed pre-Cassiopeia GH state in repo fixture; fire migration in A; assert both A + B render same containers + members | PENDING |
| 2: pre-Cassiopeia iterations preserved | Rust integration | seed pre-Cassiopeia iteration-issues + member-stories with legacy iteration_id field; fire migration; assert each existing iteration appears with member-stories label-tagged with new format | PENDING |
| 3: loose stories → Unscheduled | Rust integration | seed loose icebox stories pre-migration; fire migration; assert all carry `iteration-Unscheduled.N` labels post | PENDING |
| 4: audit-log artifact committed | Rust integration | post-migration; assert `docs/audit-logs/2026-05-NN-cassiopeia-label-substrate-migration.md` exists with expected content shape | PENDING |
| 5: priority.json icebox-state retires | Rust integration | post-migration; assert priority.json no longer holds icebox-positional state (icebox column empty or removed) | PENDING |
| 6: migration idempotent | Rust integration | run migration twice; assert second run is no-op (no spurious label-mutations; no audit-log churn) | PENDING |

## Test ordering

Per Ryan-TDD-canon ("failing-test-first"), each test should be authored to FAIL at current HEAD pre-fix, then PASS after fix lands. The tests above target slices not-yet-shipped — they'll fail at HEAD until the respective slice lands.

## Designer-lens role in tests

Designer doesn't write tests. Designer specs:
- The user-job-shape the falsifier pins
- The observable-criteria (data-attribute, DOM order, label-set, etc.)
- Which test-axis (jsdom-fireEvent / Rust-substrate / Playwright multi-context) fits

Dev writes the code; PM audits absolute-rule; designer right-thing-checks that the test's PASS state matches the user-job promised in the body.

## Sister docs

- [index.md](index.md) — UX direction overview
- [iteration-container.md](iteration-container.md) — container shape spec
- [drag-pattern.md](drag-pattern.md) — drag rendering pattern spec
- [unscheduled-container.md](unscheduled-container.md) — Unscheduled implicit container spec
- [state-coherence.md](state-coherence.md) — cross-instance + optimistic-update + migration UX
