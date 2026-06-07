# Class D rework spec — component-reuse + dead-DOM elimination

*Designer — 2026-05-16. Revised after Ryan's component-reuse canon-check 03:13 UTC: "don't we have a story component for chore bug and feature? are we not making reusable components?" Sister to "why do the stories not have their styles like they do in other places?"*

## Why this spec exists

Ryan surfaced TWO interlocking gaps via owner-walk:
1. **Dead DOM**: empty open iterations render visual clutter for no value
2. **Component-reuse violation**: Iteration.tsx renders members via custom `.board-iteration-member` button (~30 lines hand-rolled) instead of using `StoryRow` — the canonical story-row component. This causes the iteration members to LOOK DIFFERENT from flat-column stories despite being the same data.

The visual-mismatch IS the "still don't look styled" problem. CSS-alpha-bumping (my earlier Class B spec) was the wrong fix — the right fix is component-reuse.

## Item 1 — Hide open-empty iterations

**Rule:**
- Open iteration (gh_state=open) AND 0 members → DON'T render
- Closed iteration (gh_state=closed) → render regardless of member count (historical record)
- Iteration with members → always render

**Implementation (Iteration.tsx render loop):**
```jsx
{iterations.map((iteration) => {
  const members = stories.map(...).filter(...)
  if (iteration.gh_state !== 'closed' && members.length === 0) return null
  // ... rest of container render
})}
```

Sister to today's "no dead HTML" canon.

## Item 2 — Removal tool (resolved by PM convergence)

`close_iteration` IS the removal tool. Combined with Item 1 hide-rule:
- Open-empty iteration → hidden
- PM closes empty iteration → marker closes, renders as closed-state historical marker per #1300 visual spec

No new substrate needed.

## Item 3 — StoryRow reuse for iteration-members + iteration-summary

**Current Iteration.tsx member render (hand-rolled, missing features):**
```jsx
<li className="board-iteration-member" ...>
  <button className="board-iteration-member-button" onClick={...}>
    <small className="board-story-number">#{story.number}</small>
    <span className="board-story-title">{story.title}</span>
  </button>
</li>
```

**Missing vs StoryRow:**
- StoryTypeGlyph (feature/bug/chore icon)
- StoryBadge (rework, blocked, awaiting-team, etc.)
- Epic-ref link (if parent_epic)
- ReworkGlyph
- Selected-state (aria-current)
- Drag handlers (HTML5 draggable + onDragStart + onDragEnd)
- Keyboard-reorder handler
- Data-attributes for testing (data-story-type, data-closed, data-dragging)
- proper className composition (drop-before, drop-after, blocked, etc.)

**Target Iteration.tsx member render (StoryRow reuse):**
```jsx
{members.map(({ story, position }, memberIdx) => (
  <StoryRow
    key={story.number}
    story={story}
    parentEpicTitle={
      story.parent_epic !== undefined
        ? epicTitleByNumber.get(story.parent_epic)
        : undefined
    }
    selected={story.number === selectedNumber}
    draggable={iteration.gh_state !== 'closed'}
    dragging={draggedNumber === story.number}
    dropBefore={hoveredNumber === story.number /* eager-move integration */}
    dropAfterLast={memberIdx === members.length - 1 && hoveredNumber === null}
    position={memberIdx + 1}
    total={members.length}
    onKeyboardReorder={
      iteration.gh_state !== 'closed'
        ? (direction) => { /* keyboard-reorder per-iteration */ }
        : undefined
    }
    onOpen={onOpen}
    onOpenTimeline={onOpenTimeline}
    onSelect={onSelect}
    onRowDragOver={/* eager-move + cross-iteration drag handlers */}
    onDragStartRow={() => { /* member drag start */ }}
    onDragEndRow={() => { /* member drag end */ }}
  />
))}
```

**Iteration summary similarly uses StoryRow:**
```jsx
<summary>
  <StoryRow
    story={iteration}
    selected={false}
    draggable={iteration.gh_state !== 'closed'}
    /* ... iteration-row props */
  />
</summary>
```

**Benefits:**
- Visual identity: iteration members LOOK like flat-column stories (glyph, badges, epic-ref, etc.)
- Single source of truth: StoryRow style/behavior changes propagate to iteration members
- DRY: ~30 lines hand-rolled rendering removed
- Future-proof: state-badge / hover / drag changes inherit
- Test consistency: same data-test selectors work in both contexts

**Integration considerations:**
- StoryRow expects flat-column-context props (BoardGroup-coordinated dropAt, draggable from parent). Iteration needs to PROVIDE those from its own state (`draggedNumber`, `hoveredNumber`)
- Eager-move integration: StoryRow renders drop-before/drop-after via CSS classes; iteration's eager-reorder logic computes those positions
- Keyboard-reorder: existing onKeyboardReorder in StoryRow can accept iteration-context handlers
- Cross-iteration drag: StoryRow's onDragStartRow/onDragEndRow can fire iteration-specific payload (DRAG_MIMETYPE_ITERATION_MEMBER vs DRAG_MIMETYPE)

The integration is real work but the API surface aligns.

## Item 4 — Container shell styling (lighter than before)

With StoryRow handling member visual-identity, container shell needs lighter styling than my earlier Class B over-engineered spec:
- Subtle bg-tint or border to mark the `<details>` as a CONTAINER (not part of a flat list)
- Could be left-accent-bar (3px) on the `<details>` element itself
- Padding/margin to give the container breathing-room within the icebox column

Sister to Epic.tsx card styling. Light touches, not heavy reinvention. Less CSS now since StoryRow members carry their own visual weight.

```css
.board-iteration-card {
  border-left: 3px solid var(--accent-info, #5aa5ff);  /* container-signal */
  padding-left: var(--space-2, 4px);
  margin: var(--space-2, 4px) 0;
}

.board-iteration-card[data-closed='true'] {
  border-left-color: var(--color-muted, #888);
  opacity: 0.6;
}

.board-iteration-card[data-iteration-kind='unscheduled'] {
  border-left: 3px dashed var(--color-muted, #888);
}
```

## Item 5 — Sister consideration: Epic+children pattern

Epic.tsx renders children via MiniBoard component (also a sister-pattern). If Iteration uses StoryRow directly for members, that's INCONSISTENT with Epic's MiniBoard approach. Either:
- (a) Iteration uses StoryRow directly (this spec); Epic.tsx keeps MiniBoard
- (b) Both use the same rendering pattern — extract a shared `<ContainerMembers>` component

Designer-lean: (a) for #1297 scope; (b) is a sister-enhancement post-Cassiopeia. Don't conflate scope; iteration is the immediate concern.

## Verification per Class D revised

Per designer-discipline canon:
1. Post-Class-D commit ship: capture_electron_window
2. Compare iteration-member rendering side-by-side with flat-column story rendering
3. Confirm:
   - Open-empty iterations don't render
   - Iteration members LOOK identical to flat-column stories (glyph, badges, epic-ref, etc.)
   - Container has visible accent-bar/border distinguishing it from flat list
   - Closed iteration renders with dim/strikethrough/badge per #1300 visual spec

Bug-question test reapplied at each Scenario THEN-clause.

## Behavior-change discipline

Per PM's 5th-visual-recidivism count today: designer-discipline lock-in is the actual change. capture_electron_window + visual-walk BEFORE pass_desk_check fires. Per Ryan-canon "make sure quality is high before you give it to me." Not optional.

Plus: ASK MORE QUESTIONS at designer-spec time about component-reuse + DRY. The hand-rolled `.board-iteration-member` button was a designer-spec gap (specced custom CSS instead of "reuse StoryRow"). Sister to today's framing-drift recidivism.

## Sequence

1. Class A migration race-fix (shipped — 7f8628e)
2. Class B layout-hierarchy + initial polish (shipped — b376b25)
3. Class A2 historical-marker fix (shipped — 3a0a11e)
4. **Class D this spec** — Items 1 (hide-empty) + 3 (StoryRow reuse) + 4 (lighter container shell)
5. Re-deliver #1297 + designer right-thing audit per discipline
6. Owner-walk + #1301 final retirement

## Sister lessons (no new save — behavior-change canon)

- StoryRow-reuse-not-hand-roll is sister to today's substrate-pivot recidivisms
- Designer-spec discipline gap: I specified custom CSS instead of "reuse existing components"
- The pattern continues: designer should ask "is there an existing component for this?" before specing custom rendering
