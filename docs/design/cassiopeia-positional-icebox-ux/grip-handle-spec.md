# Grip-handle visual + interaction spec

*Designer — 2026-05-16. For #1339 Scenario 4 rework. PM-ratified option (a) implement grip-handle 12:55 UTC.*

## Why this spec

#1339 body Scenario 4 binding contract:

> grip-handle is visually distinct (dotted-grid icon, grab cursor on handle only — row body click opens drill-in)

Sister-clauses in body:

- User-job: "grip-handle as drag-initiator"
- Observation falsifier: "the grip-handle isn't visually distinct on draggable rows" → reject

Current StoryRow.tsx makes the entire `<li>` draggable; `cursor: grab` applies to the whole row (`Board.css:201-207`). Click on row-body opens drill-in via inner `<div role="button">`. The drag-initiator and click-affordance overlap — visually indistinguishable.

## Shape

**Position:** grip-handle is the LEADING child of `<li>`, before `#1234` number / glyph / title. Reads left-to-right as "drag-here → identity → action."

**Icon:** dotted-grid (6 dots in 2×3 grid). Conventionally the only "this is draggable" affordance in modern UX (Notion / Trello / Linear).

**Hit-target:** ≥16px square (per Norman/Nielsen reachable-touch-target guidance), padded so visual icon is ~10px but the draggable region extends to row vertical-edges.

**Cursor scope:**

- `cursor: grab` on `.board-story-grip` ONLY
- `cursor: grabbing` on `.board-story-grip:active`
- `<li>` retains default cursor (or pointer via inner `.board-story-button`)
- Critically: REMOVE `cursor: grab` from `&[draggable='true']` in Board.css:201-207 — the row-level rule must go away

**Draggable scope migration:**

- `draggable` attribute moves from `<li>` to `.board-story-grip`
- `onDragStart` + `onDragEnd` move with it
- `onDragOver` STAYS on `<li>` (drop-target, distinct concern)

## Markup target

```tsx
<li
  className={classes.join(' ')}
  aria-current={selected ? 'true' : undefined}
  data-test={`board-story-${story.number}`}
  data-story-type={type}
  data-closed={story.gh_state === 'closed' ? 'true' : undefined}
  data-dragging={dragging ? 'true' : undefined}
  onDragOver={onRowDragOver}
>
  {draggable && (
    <span
      className="board-story-grip"
      data-test="story-grip"
      draggable={true}
      role="button"
      aria-label={`Drag to reorder #${story.number}`}
      tabIndex={-1}
      onDragStart={(event) => {
        event.dataTransfer.setData('text/bpmux-story', String(story.number))
        event.dataTransfer.effectAllowed = 'move'
        onDragStartRow?.()
      }}
      onDragEnd={() => onDragEndRow?.()}
    >
      <GripIcon />
    </span>
  )}
  <div role="button" tabIndex={0} className="board-story-button" /* ... */>
    {/* unchanged inner content */}
  </div>
</li>
```

## Icon shape

```tsx
const GripIcon = (): React.JSX.Element => (
  <svg
    className="board-story-grip-icon"
    width="10"
    height="14"
    viewBox="0 0 10 14"
    aria-hidden="true"
  >
    <circle cx="2" cy="3" r="1" />
    <circle cx="8" cy="3" r="1" />
    <circle cx="2" cy="7" r="1" />
    <circle cx="8" cy="7" r="1" />
    <circle cx="2" cy="11" r="1" />
    <circle cx="8" cy="11" r="1" />
  </svg>
)
```

Inline SVG keeps the icon themable via `fill: currentColor` and avoids an icon-font dep.

## CSS

```css
.board-story-grip {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 100%;
  flex-shrink: 0;
  cursor: grab;
  color: var(--color-muted, #888);
  padding: 0 var(--space-1, 2px);
}

.board-story-grip:active {
  cursor: grabbing;
}

.board-story-grip:hover .board-story-grip-icon {
  color: var(--fg-primary);
}

.board-story-grip-icon {
  fill: currentColor;
  pointer-events: none; /* events bubble to .board-story-grip */
}

/* RETIRE the row-level cursor: grab — leading-grip is the only drag affordance */
.board-story-row[draggable='true'] {
  /* delete this whole block */
}
```

## Iteration container summary-row

For iteration-itself drag (Scenario 3), the iteration's `<summary>` also needs a grip-handle. Same shape, same icon, leading position inside `<summary>`. Iteration.tsx renders the iteration as a `<details>` with a custom `<summary>` — add `<span class="board-iteration-grip">` mirror.

## Closed iteration

Closed rows + closed iterations: `draggable={false}` precondition already excludes the grip-handle render (`{draggable && ...}` predicate). Visually closed rows have no grip → reads correctly as non-draggable.

## RTL falsifier shape

Body promises three observable claims; pin each:

1. **Grip visually distinct:** assert `getByTestId('story-grip')` exists on draggable rows, absent on `draggable=false` rows
2. **Cursor scope:** assert `getComputedStyle(grip).cursor === 'grab'` AND `getComputedStyle(row).cursor !== 'grab'`
3. **Row body click opens drill-in:** assert click on `.board-story-button` fires `onOpen(number)`; assert click on `.board-story-grip` does NOT fire `onOpen`

These three together pin the contract dev needs to implement.

## Accessibility

- `aria-label="Drag to reorder #1234"` on the grip
- `tabIndex={-1}` on the grip — keyboard-reorder is already on the inner `<div role="button">` via Ctrl+Shift+Up/Down. Don't dual-path the keyboard surface; mouse-only on grip
- `aria-hidden="true"` on the SVG — the icon is decorative; the grip's aria-label carries the affordance text

## Closed-state visual sanity

When `draggable=false` (closed row / closed iteration), the grip is absent. The row's leading edge starts directly with `#1234`. This is the correct read — non-draggable rows shouldn't pretend to be draggable.

## Sister considerations

- Drag-handle pattern is sister to Notion's drag-affordance + Trello's card grip + Linear's row grip — all use leading-edge dotted-grid. Pattern is canonized in modern productivity UI
- Per Norman's "discoverability" principle: a visible grip-handle is more discoverable than "the whole row is draggable, you just have to know." The leading-edge dotted-grid is a learned affordance
- I haven't read the ChosenPicachu source directly — the body's grip-handle clause is binding regardless of what ChosenPicachu does. Per `feedback_ground_cited_references_before_spec_claims`, I'm explicitly marking that grounding-gap

## Implementation sequence

1. Add `.board-story-grip` span + GripIcon to StoryRow.tsx; move `draggable` + drag handlers from `<li>` to grip
2. Add CSS rules; RETIRE Board.css:201-207 row-level `cursor: grab`
3. Mirror in Iteration.tsx for iteration-itself drag
4. Add RTL test for the three falsifier-shapes above
5. Run existing iteration-drag-*.test.tsx — they should still pass (drag-start fires from the grip; rest of payload unchanged)
6. Capture electron window screenshot for visual evidence at desk-check
