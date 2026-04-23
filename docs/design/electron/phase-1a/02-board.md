# phase-1a board surface

## toggle + layout

- opens via **Ctrl+P** (primary) or **View → Board** menu
- closes via Ctrl+P (toggle) or Esc when board focused
- right-docked panel; chat stays visible to left (collaboration invariant)
- fixed ~360 cols; hides below 800-col window width

## default view (story list)

```
┌─ Board ──────────────────────────┐
│ ★ Ready (3)                      │
│   #324 ✨ [Feature] Side-panel…  │
│   #325 ✨ [Feature] Project pnl… │
│   #326 🐛 [Bug] …                │
│ ★ In Progress (1)                │
│   #327 🔧 [Chore] Workspace…     │
│ ★ Delivered (2)                  │
│   #330 ✨ [Feature] …            │
│ ★ Accepted (collapsed, click)    │
└──────────────────────────────────┘
```

- **state groups**: Ready / In Progress / Delivered / Accepted (collapsed) / Icebox (collapsed). Awaiting-Accept retired per #359 until phase-2 team-accept-badge (#D) lands with real semantics — reintroduce renamed per observed behavior at that point, not pre-reserved vocabulary.
- **per row**: `#NNN` (dim) + type glyph (✨/🐛/🔧 Feature/Bug/Chore) + title (truncate with ellipsis; full title on hover)
- **selection**: arrow keys move highlight; Enter opens drill-in; Esc closes panel (if no drill-in open)
- **drag-reorder**: within Ready, drag to reposition; emits reorder_story to backend
- **accepted group**: collapsed by default; click header to expand (reduces visual noise for long session)

## drill-in (replaces list while open)

```
┌─ Board · #325 ◀ back ────────────┐
│ ✨ [Feature] Project panel drill…│
│ state: ready                     │
│ labels: Feature, ready           │
│ assignees: (none)                │
│                                  │
│ ## What                          │
│ body text, plain-rendered,       │
│ wraps at panel width, scrollable │
│                                  │
│ ## Why                           │
│ …                                │
│                                  │
│ ## Acceptance criteria           │
│ …                                │
│                                  │
│ [Accept] [Reject]  ← if owner+delivered
└──────────────────────────────────┘
```

- **header**: `Board · #NNN ◀ back` — click back or Esc returns to list; selection preserved
- **metadata**: state + labels + assignees
- **body**: plain text rendering in phase-1a (markdown deferred to 1b); scrollable if long
- **action buttons** (bottom, owner-only): Accept / Reject visible when story is delivered AND current user is ryan
- **phase-1b defers**: comments thread, timeline audit, cross-story nav, markdown+code-highlight, team-accept badges

## drag-reorder behavior

- visible only in Ready column
- grab handle on left edge of row (or drag whole row)
- drop between two rows inserts; drop on row swaps
- reorder_story MCP fires on drop → backend updates → board_invalidate refreshes

## states

- **loading**: skeleton rows (~3 per state group) with shimmer; replace when first fetch lands
- **empty state**: each group shows "none" dim text when empty
- **fetch error**: non-modal warning strip + retry button; keeps last known state visible
- **stale indicator**: none in phase-1a (board poller refreshes every N seconds; real-time signal via BOARD_INVALIDATE_MARKER on mutations)

## keyboard nav

- ↑/↓: move selection
- ←/→: collapse/expand state group
- Enter: open drill-in (if row selected)
- Esc: if drill-in open → back to list; else → unfocus panel; else → close panel
- Ctrl+P: toggle panel (global)

## click affordances

- row click: open drill-in
- state-group-header click: collapse/expand
- `◀ back` link: close drill-in → list
- empty-area click within panel: unfocus to chat (matches collaboration invariant)
