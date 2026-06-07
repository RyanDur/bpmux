# Drag-reorder visual contract (all sortable lists)

Designer-owned canonical contract for the drag-look of EVERY `.sortable-list` consumer
(ready-queue #1524, iteration-rows #1517, member-list, and future). One shared mechanism +
shared CSS → uniform by construction. Lives in this doc (not story-comments) so it's readable
at-source — the same lesson as the reject-context fix.

History: the original contract (#1524, 2a790c0f) specced the source-placeholder but OMITTED the
drag-ghost. That gap surfaced at Ryan's 2026-06-06 real-mouse walk of #1517 — "the row image
that should be holding disappears when I am holding it on drag… the actual element should be
hidden but there should be a visible image of the row I am dragging connected to the pointer."
This contract closes the gap.

## The visual has TWO decoupled parts

A correct drag-reorder shows two things at once (Figma / Linear / Notion / every good
reorder):

### 1 · SOURCE placeholder (in-list) — the gap
The dragged element, in its list slot, goes to `opacity: var(--invisible-placeholder)` (`0.001`)
— effectively hidden, leaving a gap that shows where the row will drop. Shared rule:
`.sortable-list [data-dragging='true'] { opacity: var(--invisible-placeholder); }` (Board.css).
This is correct and stays. (`0.001` not `0` keeps layout/hit-geometry for the drop target.)

### 2 · DRAG-GHOST (on the pointer) — what you're carrying
An EXPLICIT, **full-opacity image of the row**, attached to the pointer, following it as you
drag. This is the "I'm holding this" affordance Ryan named. It must be VISIBLE and decoupled
from the source's opacity.

**Why it broke:** HTML5 native DnD builds the default drag-image by *snapshotting the source
element* at `dragstart`. With no explicit drag-image and the source already at opacity `0.001`,
the snapshot captures the near-invisible element → blank ghost. The source-hide and the ghost
were coupled through the default-snapshot.

**The fix (mechanism is dev's; the OUTCOME is the contract):** set an explicit drag-image —
`event.dataTransfer.setDragImage(clone, x, y)` with a **full-opacity clone of the row** at
`dragstart`, in the shared `useDragReorder` path. This guarantees a visible ghost regardless of
the source opacity (robust; does not race React's `data-dragging` timing against the synchronous
snapshot). One fix in the shared path cures #1517 + #1524 + every sortable list.

**The ghost is a SOLID, picked-up card — give the clone an EXPLICIT surface.** The row at rest is
`background: transparent` (Board.css `.board-iteration-card`) — it *borrows* the board surface
behind it. A clone lifted off the board has nothing behind it, so only the text (which carries
its own color) survives → "words stay, background disappears" (Ryan's 2026-06-06 walk, round 2).
Principle: **a lifted element must make explicit the surface it was borrowing, plus elevation.**
The out-of-tree clone must make its surface EXPLICIT (it no longer borrows the container).
Two viable ways, both shipped/considered on #1517:
- **(a) Copy the PAINTED styles PER-ELEMENT** (shipped, 1d01c21b, Ryan-confirmed): `copyPaintedStyles`
  walks the clone tree and copies each element's `getComputedStyle` `backgroundColor` / `color` /
  `borderColor` inline. THE TRAP it must avoid: copying only the *card's* computed background gives
  transparent (`.board-iteration-card { background: transparent }` → `getComputedStyle` returns
  transparent — the computed value is the element's own, not the visual composite of what shows
  through). Copying PER-ELEMENT recovers a visible surface because child elements carry paint.
- **(b) An EXPLICIT lifted surface** — `background: var(--bg-elevated)` + `border-subtle` +
  `border-radius` + a moderate lift `box-shadow` — deterministic, and reads more "picked up."
Either yields a visible, self-contained card; (a) is what shipped + the owner approved.

**The clone is rendered OUT of the React-managed list tree** (a detached node, or a portal to
`document.body`) — NEVER parked inside the live list container. A foreign DOM node inside React's
tree disrupts reconciliation on the very next render after the drop → the dropped row HIDES until
a refresh rebuilds the list (the "disappears-until-refresh" vanish, #1524 2026-06-06). The
order persists underneath; it is purely a post-drop render glitch from an in-tree ghost. So:
out-of-React-tree clone + explicit lifted surface = visible ghost, no vanish.

### 3 · Grip affordance
`.board-story-grip:focus-visible, .board-iteration-grip:focus-visible` carry the identical
`outline: 2px solid var(--accent-brand); outline-offset: 1px` ring (keyboard affordance). Shared.

## Uniform by construction
All consumers render through `SortableList` (its `<ul>` carries the `sortable-list` base class),
so the source-placeholder rule + the shared drag-ghost mechanism + the grip ring apply
identically. No per-surface drag CSS — divergence is a regression.

## Observation-falsifier
I would reject if: (1) the drag-ghost on the pointer is invisible / blank / near-transparent;
(2) the ghost is not a full-opacity image of the row being dragged; (2b) the ghost shows the
row's text but no solid surface (transparent/missing background — the lifted card must carry an
explicit `--bg-elevated` surface + border + lift-shadow); (3) the source element in
the list is NOT hidden (no placeholder gap) during the drag; (4) the ghost or placeholder
diverges between sortable lists (ready-queue vs iteration-rows vs member-list) — must be one
shared mechanism; (5) the grip lacks the shared focus-visible ring. The live ghost + felt is a
real-OS-DnD behavior — Ryan's real-mouse walk is the load-bearing catch (a static capture and a
Playwright drag are both blind to the snapshot-time ghost).
