# Drag pattern: eager-move + cross-container

*Designer — 2026-05-15. ChosenPicachu HideElemOnDragSortableListEagerMove + drag-across-container semantic.*

## Two drag axes

The Cassiopeia substrate makes drag the membership-and-position ceremony:

1. **Within-container drag** — reorder stories within an iteration (or within Unscheduled). Updates position-suffix on the story's label.
2. **Cross-container drag** — move a story from one container to another. Updates iteration-prefix on the story's label (TagRemoved + TagAdded canonical-mutation-chain).

Plus a tertiary axis:

3. **Iteration-reorder drag** — drag an iteration's summary-row to reposition the iteration itself among iterations. Updates the iteration-issue's `iteration-order-NNNN` label.

## The eager-move pattern (within-container)

When the user initiates a drag on a row, the row HIDES from its original position. As the user moves the pointer through the container's body, the OTHER rows shift in real-time to make room at the prospective drop position. The user sees the future-state of the list during the drag. On drop, the list is already in the final position — substrate-mutation fires to persist.

This matches ChosenPicachu HideElemOnDragSortableListEagerMove (referenced in #1339 Source). WYSIWYG-during-drag.

### Eager-move is a rendering pattern, not an event-API choice

Two distinct axes are sometimes conflated:

1. **Event API** — what fires during drag. Two options:
   - HTML5 drag API: `draggable={true}` + onDragStart/onDragOver/onDrop
   - Pointer API: mousedown/mousemove/mouseup (or pointerdown/move/up)
   - Either API can power either rendering style below

2. **Rendering during drag** — what user SEES while pointer moves. Two patterns:
   - Browser-default rendering on HTML5 events = semi-transparent ghost image + other rows stay STATIC + drop-indicator line shows landing spot
   - Eager-move rendering (whichever event API) = hide original element + DYNAMICALLY reorder other rows live as pointer crosses midpoints + drop just confirms what's visually already true

The currently-shipped Cassiopeia attempt uses HTML5 events + browser-default rendering (event-API #1 + rendering #1). Eager-move keeps the same event API but swaps the rendering — typically by setting `opacity: 0` or `display: none` on the dragged element in `onDragStart` and live-reordering the visible list in `onDragOver`. The browser's ghost-image is suppressed by a custom drag-image (or by an early hide-original).

ChosenPicachu's `HideElemOnDragSortableListEagerMove` is the rendering-pattern; whether the underlying events are HTML5 drag or pointer is implementation-detail.

### Why eager-move rendering is what shipped wrong the first time

The first Cassiopeia attempt rendered with browser-default ghost + static rows. Ryan's body-promise for #1339 sourced ChosenPicachu's eager-move rendering as the visual reference. The first attempt missed that the visual rendering needed to be custom, not browser-default.

### Interaction states

| State | Visual |
|-------|--------|
| Idle | Row in normal position; grip-handle on left visible on hover for draggable rows; cursor: `grab` on hover |
| Drag-initiated | Row hides from origin; other rows shift to fill the gap; cursor: `grabbing`; drop-zone highlighting begins |
| Drag-moving-within-source | Source-container's list reshapes eagerly as pointer crosses row midpoints |
| Drag-moving-cross-container | Source-container's body releases the row (gap closes); receiving-container's body opens a slot at prospective drop position |
| Drag-over-invalid-zone | List(s) return to pre-drag visual; cursor: `not-allowed`; (e.g., dragging into a closed iteration's body) |
| Drop | List stays in final position; canonical-mutation fires; on persist-fail, list animates back to pre-drag state with visible error indicator |

## Cross-container drag affordance

When drag-pointer crosses from container A's body region into container B's body region:

1. Container A's list shifts to close the gap left by the dragged row
2. Container B's body region highlights with a drop-zone affordance:
   - Border accent-color glow (`box-shadow: inset 0 0 0 2px var(--accent-info)`)
   - Brief subtle pulse-animation (200ms) on container-entry
   - Within B's body, a slot opens at the prospective drop position (eager-move applies)
3. Container A's body de-highlights as pointer leaves
4. If user drags BACK into A, the slot opens in A's body again
5. On drop: substrate-mutation fires (TagRemoved old-iteration-prefix; TagAdded new-iteration-prefix; position-suffix calculated for the drop position in B)

### Closed-container drop-rejection

If user attempts to drop into a closed iteration's body:
- Container shake-animation (200ms horizontal shake)
- Cursor remains `not-allowed` during the hover
- Drop event does not fire substrate-mutation; row returns to source position
- Brief visible message near the closed-iteration summary: "Closed iterations cannot be re-scoped"

## Iteration-reorder drag (drag iteration itself)

The iteration's `<summary>` row has a grip-handle on the left. Drag the summary-row to reposition the iteration among iterations:

- Eager-move applies (the iterations section is its own SortableList of `<details>` containers)
- Other iterations shift as pointer crosses midpoints
- On drop: substrate-mutation fires on the iteration-issue's `iteration-order-NNNN` label
- Cross-iteration drag is INSIDE the iterations-section bounds only (can't drag an iteration into the loose-stories region; Unscheduled is always-last)

### Reorder lock

- Closed iterations cannot be reordered (grip-handle hidden or disabled)
- Unscheduled cannot be reordered (grip-handle hidden; always-last position)

## Accessibility (keyboard parity)

Each draggable element supports:
- `Ctrl+Shift+ArrowUp` / `Ctrl+Shift+ArrowDown`: move one position in the same direction (existing pattern per StoryRow.tsx)
- For cross-container moves via keyboard: `Ctrl+Shift+ArrowRight` / `Ctrl+Shift+ArrowLeft` could move story to next/prev container (sister to within-container keyboard nav). Designer-judgment: ship within-container keyboard-reorder first; cross-container keyboard-shortcut as enhancement.
- `Escape`: cancel in-progress drag
- Live-region announcement: "Moved <story-title> to position N of M in <container-name>" (sister to existing setReorderAnnounce)

## Grip-handle visual

- 12×12 dotted-grid SVG icon (canonical sortable-list grip pattern)
- Color: `var(--fg-dim)` at rest; `var(--fg)` on hover
- Cursor: `grab` on the handle specifically; `default` on rest of row (click opens drill-in)
- Visible always on iteration summary-rows + Unscheduled summary-row
- Visible on-hover on story-rows (drag is supported but not the primary affordance)

## Out of scope

- Touch gestures (mobile/tablet)
- Multi-row drag-select
- Drag from icebox to other columns (cross-state drag); state-transitions still happen via close_iteration ceremony, start_story, etc.

## Reference implementation

ChosenPicachu — concrete code at `src/components/About/DragAndDrop/`:

- `HideElemOnDragSortableListEagerMove.tsx` — the primary reference pattern
- `Draggable.tsx` — wrapper component (drag-state + grip-handle)
- `HideOnDrag.tsx` — hide-original-during-drag wrapper
- `SortableListEagerMove.tsx` — eager-move WITHOUT hide-on-drag (for compare)
- `HideElemOnDragSortableListLazyMove.tsx` + `SortableListLazyMove.tsx` — lazy variants (reorder on `onDragEnd`, not `onDragOver`)
- `styles.css` + `styles.layout.css` — visual treatment
- `grip.svg` — grip-handle icon

Concrete pattern bpmux should adopt (sourced from reading the actual code):

1. **HTML5 drag events** on a `Draggable` wrapper — `onDragStart`, `onDragOver`, `onDragEnd`
2. **Grip-handle as drag-initiator** — only grip's `mouseDown` enables `draggable={true}`; row body click opens drill-in instead
3. **Hide-original via CSS class** — `onDragStart` adds a `.hide` class; CSS `&.hide { opacity: 0.1% }` makes the element effectively invisible while preserving layout space; `onDragEnd` removes the class
4. **Eager reorder on `onDragOver`** — `if (draggedItem) updateList((oldList) => moveToIndex(index, draggedItem, oldList))` — re-orders the array immediately on hover; React re-renders; the hidden dragged element appears as a "gap" at its prospective position
5. **Visual elevation on grip-hover** — CSS `:has(.grip:hover) { box-shadow: 0 10px 14px rgba(0, 0, 0, 0.12) }` signals "this is grabbable"
6. **`array.moveToIndex(targetIndex, item, oldList)` utility** — remove item from list, insert at targetIndex (extract or replicate this helper)

Dev's discretion on whether to vendor (copy files), import-as-dependency (if ChosenPicachu publishes), or re-implement matching the pattern. bpmux should match the feel concretely; the reference code makes "matching the feel" verifiable.

### Cross-container extension (bpmux-specific)

ChosenPicachu's pattern is single-list. bpmux extends to multi-container drag (drag a story from iteration A's body into iteration B's body). Extension:

- Each container (iteration `<details>` body) is its own `currentList` state
- A higher-level `draggedItem` + `sourceContainer` state coordinates across containers
- `onDragOver` on a row inside container B (with draggedItem from container A) triggers:
  - Remove draggedItem from container A's `currentList`
  - Insert draggedItem into container B's `currentList` at the hovered row's index
- The drag-end fires the canonical-mutation-chain for the label-prefix change (TagRemoved + TagAdded)
- Closed containers reject `onDragOver` (no state mutation; cursor `not-allowed`)

The hide-original applies as in the single-list pattern — dragged element invisible at its current array position (in either container).
