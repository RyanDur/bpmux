# Iteration container

*Designer — 2026-05-15. `<details>` collapsible per iteration + collapse-state persistence + drag-between-containers semantic. Sister to existing Epic.tsx pattern.*

## Shape

Each iteration in the icebox renders as a collapsible container, mirroring the Epic.tsx pattern (`applications/electron/src/renderer/src/board/Epic.tsx:75-131`).

```
<li class="board-iteration-item">
  <details
    class="board-iteration-card"
    data-test="board-iteration-1338"
    open={open}
    onToggle={...}
  >
    <summary class="board-iteration-summary">
      <button> (open drill-in)
        <glyph type="iteration" />
        <small>#1338</small>
        <span data-state="open">open</span>
        <b>Cassiopeia</b>
      </button>
      <progress value={done} max={total} />
      <small>{done}/{total} ✓</small>
    </summary>
    <div class="board-iteration-body">
      <p class="board-iteration-description">...</p>
      <ul class="board-iteration-stories">
        {members ordered by label-position-suffix}
      </ul>
    </div>
  </details>
</li>
```

### Summary row

- **Grip-handle** (left-most): drag the iteration itself to reorder among iterations
- **Disclosure triangle**: `<details>` browser-native (or styled chevron)
- **Iteration glyph**: flag-on-pole SVG (already in StoryTypeGlyph.tsx)
- **Number** (`#1338`): canonical issue reference
- **State badge**: `open` / `closed` (closed-state has muted treatment)
- **Title** (`Cassiopeia`): iteration name, primary identifier
- **Progress bar**: completed members / total members (sister to Epic progress)
- **Progress text**: `{accepted}/{total} ✓` (sister to Epic summary)

### Body

- **Description**: first paragraph of iteration-issue's body (extracted via `extractDescription`, sister to Epic)
- **Member list**: ordered by label-position-suffix; each member is a regular StoryRow
- Drag-affordance INSIDE the body: each story has grip-handle on hover; eager-move within container

### Collapse defaults

- **Open iterations**: open by default (sister to Epic `isOpenByDefault` heuristic — not in icebox/accepted)
- **Closed iterations**: closed by default (collapsed; show just the summary as historical record)
- **Empty iterations** (no members yet): closed by default; summary visible

### Collapse-state persistence

Per-user-local, sister to existing `useEpicCollapse` hook (`applications/electron/src/renderer/src/board/useEpicCollapse.ts`):
- Keyed by RENDERER_IDENTITY (per-user)
- localStorage-backed, cross-session
- NOT cross-instance (collapse is a viewing preference, not canonical fact)

Implementation: generalize `useEpicCollapse` to `useContainerCollapse(kind: 'epic' | 'iteration', number)` OR mirror as `useIterationCollapse`. Either pattern works.

## Iteration ordering

Iterations themselves are ordered within the icebox by an iteration-level label on the iteration-issue: `iteration-order.0`, `iteration-order.1`, ... (dot separator + dense-index, sister to per-story position-suffix).

The iterations section is its own SortableList: drag an iteration's summary-row (via grip-handle) to reorder iterations relative to each other.

Closed iterations: still sort by `iteration-order-NNNN` tag. Closed-state visual signifier dims them but doesn't move them. Closed iterations could be filtered out via a toggle ("show closed iterations") in the icebox header — out-of-scope for v1; v1 shows all iterations regardless of state.

## Drag-between-containers semantic

When a story is dragged from iteration A's body to iteration B's body:

1. **Visual** — row leaves A's body (A's list reshapes to close the gap); arrives in B's body at prospective drop position (B's list reshapes to accommodate)
2. **Drop-target hint** — as drag-pointer enters B's body region, B's border highlights (`var(--accent-info)` glow); A's body de-highlights
3. **Closed-iteration drop-rejected** — if user attempts to drop into a closed iteration's body, the drop is REJECTED:
   - Cursor: `not-allowed`
   - Closed-iteration shake-animation (200ms horizontal shake)
   - Row returns to source position
4. **Substrate-mutation**: drag-end fires TagRemoved(`iteration-#A-XXXX`) + TagAdded(`iteration-#B-YYYY`) via canonical-mutation-chain (existing primitive)
5. **Position-suffix calculation**: dev-axis; sparse-numeric "insert between" produces `(prev + next) / 2`

Sister-affordance: drag from Unscheduled into a named iteration = "scheduling a story." Drag from a named iteration into Unscheduled = "un-scheduling." UX-cohesive.

## Drag-iteration-itself

The iteration's summary-row is also draggable (via the grip-handle in the summary):
- Drag to reorder iterations within the icebox iterations-section
- Eager-move applies (within the iterations SortableList)
- Substrate: update the iteration-issue's `iteration-order-NNNN` label

## Closed-state visual

`details.board-iteration-card[data-closed="true"]` (where `data-closed` reflects iteration-issue gh_state==='closed'):
- Summary opacity: `0.65`
- Title: `text-decoration: line-through; color: var(--fg-dim)`
- Iteration glyph color: `var(--fg-dim)` (dim, not accent)
- Summary background: `var(--bg-subtle)` (muted vs open-iteration accent-tinted bg)
- Grip-handle: hidden (iteration-order can still be changed, but typically not for closed iterations; designer-judgment: hide for v1 to signify "frozen historical record")
- Body (when expanded): members visible but ALSO dimmed; drag inside body locked (closed iteration's members are frozen too)

## Empty container

If an iteration has zero members:
- Summary visible
- Body shows: "No stories in this iteration. Drag stories here to add them." (Norman-DOET-style empty-state instruction)
- Drop-target affordance still works — drag any story into the empty body to add

## Sister patterns

- **Epic+children** (Epic.tsx): same `<details>` collapsible pattern. Epic-grouping is parent-epic-axis; iteration-grouping is iteration-membership-axis. Both coexist; a story can have both `parent-epic-#NNN` and `iteration-#MMM-XXXX` labels.
- **isClosedIterationMarker** (types.ts:172): predicate already exists for closed-iteration detection; reuse for closed-state UI gating.
- **MiniBoard** (already used for Epic children): could be reused for iteration members, OR a simpler `<ul>` of StoryRows fits since iteration-members are flat-ordered (no state-bucketing). Designer-judgment: flat `<ul>` is simpler-cohesive.
