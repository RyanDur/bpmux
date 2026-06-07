# Class B rework spec — layout-hierarchy + visual polish

*Designer — 2026-05-16. Body-promise gaps surfaced via Ryan's owner-walk on rebuild post-#1301. Covers 5 sub-gaps + sister-canon correction.*

## Why this spec exists

Ryan's owner-walk on rebuild surfaced 4 visible body-promise gaps:
1. **Layout-hierarchy gap**: iteration containers render at TOP-LEVEL (alongside EPICS), NOT inside the ICEBOX column section. Conceptual-model violation per Ryan-canon 2026-05-15 12:23 UTC: "the icebox would be made up of iterations and loose stories" — iterations are CHILDREN of icebox.
2. **Open-by-default empty-state UX**: all 5 open iteration containers default-expanded with count=0. Empty containers as visual clutter; Norman-DOET progressive-disclosure violated.
3. **Strikethrough-through-closed-badge double-encode**: closed iteration's "(closed)" badge inherits parent's `text-decoration: line-through`. Aesthetic redundancy + reduces badge readability.
4. **Title + count visual mash**: "Pegasus0" / "Cassiopeia0" / "Unscheduled144" — count rendered adjacent to title with no space. UX-ambiguous (count reads as part of name).
5. **Container-vs-row visual distinction weak**: iteration `<details>` containers don't visually read as containers; no border, no background tint, just glyph + text — visually similar to flat StoryRows.

## Designer correction on earlier spec

My earlier `index.md` framed: "post-#1339, iteration-containers + Unscheduled-container ARE the icebox" and recommended removing the flat ICEBOX column at 14:54 UTC. **That framing was wrong.** Ryan's mental model per 12:23 UTC: iterations are CHILDREN of icebox column, not replacement. ICEBOX column STAYS as parent container; iteration-containers + Unscheduled live INSIDE it.

The earlier spec docs need correction (separate task post-Class-B ship).

## Spec — layout-hierarchy fix

**Current Board.tsx layout:**
```
<section className="board-body">
  <Epic ... />
  <Iteration ... />     ← top-level; outside any column-section
  {GROUP_ORDER.map(group => <BoardGroup ... />)}  ← icebox column renders here, empty
</section>
```

**Target Board.tsx layout:**
```
<section className="board-body">
  <Epic ... />
  {GROUP_ORDER.map(group =>
    group.state === 'icebox'
      ? <BoardGroup ...>
          <Iteration ... />   ← rendered INSIDE icebox column body
        </BoardGroup>
      : <BoardGroup ... />
  )}
</section>
```

ICEBOX column section header stays visible. Iteration-containers + Unscheduled render WITHIN the icebox-group's body. Single visual source-of-truth for icebox content.

Alternative shape: `<BoardGroup state="icebox">` always renders the `<Iteration>` block in its body (no flat StoryRow list since post-#1339 all icebox-state stories live inside iteration-containers).

Dev-call on the cleanest implementation; designer-intent is "icebox is the parent container; iterations + Unscheduled live inside."

## Spec — empty-state default

**Current behavior:**
```javascript
const defaultOpen = iteration.gh_state !== 'closed'
```
All gh_state=open iterations expand by default regardless of member-count.

**Target behavior:**
```javascript
const defaultOpen = iteration.gh_state !== 'closed' && members.length > 0
```
Empty iterations default-collapse to reduce visual clutter. Sister-canon to Norman-DOET progressive-disclosure: don't show empty content.

OR alternative: keep default-open + render explicit empty-state message inside body:
```
<ul className="board-iteration-body">
  {members.length === 0
    ? <li className="board-iteration-body--empty">Drag stories here to schedule them</li>
    : members.map(...)
  }
</ul>
```

Designer-lean: **default-collapse for empty**. Cleaner; less screen real-estate; user can expand to see "drag stories here" affordance OR just drag a story into a collapsed iteration's summary-row (closed-state visual cue).

Note: Unscheduled defaults-OPEN regardless (per existing `collapse.isOpen(UNSCHEDULED_KEY, true)`) since it's the orphan-bucket and users want to see what's there.

## Spec — strikethrough fix

**Current CSS:**
```css
.board-iteration-card[data-closed='true'] .board-iteration-title {
  text-decoration: line-through;
}
.board-iteration-card[data-closed='true'] .board-iteration-title::after {
  content: ' (closed)';
  color: var(--color-muted, #888);
  font-weight: normal;
}
```
The `::after` content INHERITS the parent's `line-through` because it's a pseudo-element of the title.

**Fix option A (cleaner):**
```css
.board-iteration-card[data-closed='true'] .board-iteration-title::after {
  content: ' (closed)';
  color: var(--color-muted, #888);
  font-weight: normal;
  text-decoration: none;    /* explicit override */
  display: inline-block;     /* breaks inheritance via separate inline-formatting-context */
}
```

**Fix option B (more structural; cleanest):**
Move "(closed)" badge OUT of `.board-iteration-title` into a sibling element:
```jsx
<b className="board-iteration-title">{iteration.title}</b>
{iteration.gh_state === 'closed' && <span className="board-iteration-closed-badge">(closed)</span>}
```
CSS:
```css
.board-iteration-closed-badge {
  color: var(--color-muted, #888);
  font-weight: normal;
  font-style: italic;
  margin-left: var(--space-2, 4px);
}
```

Designer-lean: **option B** — structural separation cleaner long-term + makes the badge a first-class element that can carry its own a11y attributes if needed.

## Spec — title + count gap

**Current:**
```jsx
<b className="board-iteration-title">{iteration.title}</b>
<small className="board-iteration-count">{members.length}</small>
```
Renders "Pegasus0" without space.

**Target:**
```jsx
<b className="board-iteration-title">{iteration.title}</b>
<small className="board-iteration-count">({members.length})</small>
```

OR via CSS flex-gap:
```css
.board-iteration-summary {
  display: flex;
  align-items: baseline;
  gap: var(--space-2, 4px);
}
```

Designer-lean: paren-wrap count "(N)" is more explicit + reads as a count indicator. Sister to Epic.tsx pattern (`<small>{summaryProgress}</small>` shows "0/3" — count format already varies).

## Spec — container-vs-row visual distinction

Currently iteration `<details>` containers visually look like indented StoryRows. Make them READ as containers:

**Container background + subtle border:**
```css
.board-iteration-card {
  background: var(--accent-info-bg, rgba(0, 191, 255, 0.04));
  border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.08));
  border-radius: var(--radius-2, 4px);
  padding: var(--space-2, 4px) 0;
  margin: var(--space-2, 4px) 0;
}
```

Hover affordance on summary (signal that summary is interactive):
```css
.board-iteration-summary:hover {
  background: var(--accent-info-bg-hover, rgba(0, 191, 255, 0.08));
}
```

For Unscheduled, dashed-border + muted color already specified; keep existing styling.

## Sequencing + ship order

1. **Class A** (dev's option-3 + #1 cleanup TagRemoved) — urgent live-data pollution; ship first
2. **Class B (this spec)** — designer hands off to dev; ~30-50 lines Board.tsx restructure + ~25-40 lines CSS
3. **Re-deliver** #1297 + #1300 (#1339 visual treatments mostly unaffected; may need minor update if container-distinction CSS affects member-row rendering)
4. **Ryan re-walks**

## Behavior-change discipline note

Per PM 01:46 UTC: "this isn't a lesson-saving moment; the discipline is loaded. It's a behavior-change moment." Today's pattern: 4 sister-class slips with lessons saved. Behavior didn't follow.

Designer-discipline going forward at pass_desk_check fire:
1. `capture_electron_window` BEFORE the broadcast — not optional
2. Visually walk EACH body-Scenario THEN-clause against the captured rendering
3. Cite the captured-pixel evidence in the PASS broadcast OR fail
4. No "substrate-axis-verified ⇒ body-promise-met" shortcuts
5. No "tests-pass ⇒ visual-quality-met" shortcuts

This is execution-discipline, not lesson-syntax. The next pass_desk_check fires only if the rendered-visual walk-as-Ryan empirically passes.
