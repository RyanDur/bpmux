# phase-1b blocker visual indicator

Ryan 2026-04-22 ~23:51 UTC during acceptance walk: *"There are 7 items
left in delivered. Are they blocked? If they are blocked they need
some visual telling me they are blocked."*

Currently the DELIVERED column renders blocked + awaiting-accept
stories identically. Owner can't distinguish at-a-glance which are
truly-awaiting-his-accept vs waiting-on-dev-rework vs waiting-on-a-
dependency. #390 surfaces block-state in the board row visual
vocabulary.

## 3-field spec header (new retro-committed discipline)

**user-job:** When Ryan performs an acceptance walk through DELIVERED,
he needs to distinguish blocked stories from truly-awaiting-accept
stories at-a-glance, because blocked stories are not actionable-for-
him — they need dev unblocking before they can reach his gate.

**observation-falsifier:** I would reject if, looking at DELIVERED,
I cannot tell within ~1 second which stories are blocked without
hovering/clicking, OR if blocked stories are hidden (violating the
nothing-hidden-by-default principle).

**team-shape-independence:** blocker visual derives from story labels
(`blocked`, `blocked-by:<N>`), not from any hardcoded role list. Works
for any agent composition.

## principle alignments

- **nothing-hidden-from-owner** (2026-04-22 principle): blocked rows
  stay visible in DELIVERED; not collapsed into a "Blocked (N)"
  section owner has to expand. Distinct visual, not hidden.
- **shape + color, not color-alone** (phase-1a a11y): block indicator
  is a distinct glyph + color, passes WCAG AA contrast, colorblind-
  readable.
- **state-aware badge substrate extension** (#368/#375/#376): block
  indicator rides the existing `StoryBadge` surface rather than
  inventing a new row-level affordance.

## layout

```
┌─ DELIVERED (7) ─────────────────────────────────────┐
│ #361  CI glyph visibility             [awaiting..]  │  ← accept-lane badge
│ #366  Comment quote-excerpt           [awaiting..]  │     (existing)
│ #367  Timeline audit-trail            [awaiting..]  │
│ ⛔     #337  Comments panel scroll    [⛔ blocked]  │  ← blocked row distinct
│ ⛔     #345  Live comment refresh     [⛔ blocked]  │     dim + icon + badge
│ #371  Right-click context menu        [awaiting..]  │
│ #358  Reject-form close animation     [awaiting..]  │
└─────────────────────────────────────────────────────┘
```

Block-state rows:
- **left-margin glyph** (⛔ or similar, see "glyph choice" below) at
  row start — preattentive anchor (position + shape) before color
- **row body dim** (opacity ~0.7) — secondary emphasis, still fully
  readable; signals "not in your queue right now"
- **badge** replaces the accept-lane badge with a block-indicator
  badge (`⛔ blocked` text + tooltip)
- **click on badge or glyph** opens the blocker drill-in (if
  `blocked-by:<N>` label present) OR the story itself (so owner can
  read the block context)

Accept-lane rows unchanged. Delivered count (7) still includes blocked
stories; they aren't hidden from the count.

## glyph choice

Author a `BlockerGlyph` as a 12x12 viewBox, `currentColor`, hand-drawn
inline SVG. (NB: `ClassGlyph.tsx` — the prior inline-SVG glyph exemplar
from #373 this once pointed at — was **retired in #1647**; there's no
live component to extend, so `BlockerGlyph` is authored fresh, with the
pattern referenceable from git history.) Reason:
unicode ⛔ rendering varies across OS font stacks; inline SVG keeps
visual consistency with the other lifecycle glyphs (insight / activity
/ system).

Proposed form: small circle with diagonal stroke (road-sign vocabulary
— universal "blocked / no-entry" semantic). Simple, 12x12 legible,
shape-distinct from ✓ (accept) and ↻ (rework).

## badge substrate extension

Extend `BadgeKind` in `StoryBadge.tsx`:

```ts
export type BadgeKind = 'team-accepted' | 'accepted' | 'rework' | 'blocked'
```

`deriveBadge` precedence (outer-to-inner override order):
1. blocked (if `blocked` label present AND state === delivered)
2. rework (if `desk-check-fail` label present)
3. accept-lane badge (existing logic)

Rationale: blocked is the most actionable owner-signal — it tells
Ryan "skip this; not actionable-for-you." Overrides everything else
on delivered rows.

`labelFor('blocked', state)`:
- delivered: `blocked — <blocker-reference-or-"awaiting-unblock">`
- accepted: `blocked` state shouldn't survive into accepted; if it
  does (data shape assumption violated), fall through to
  accept-lane label + log a console warn (defensive)

## blocker-reference

If the story carries a `blocked-by:<N>` label, the badge tooltip +
aria-label include the blocker story reference:

- tooltip: `"blocked — waiting on #337 (drill-in scroll)"`
- click-on-badge: opens #337 drill-in (same pattern as
  `onOpenTimeline` in current `StoryBadge`)

If no `blocked-by:<N>` label, fall back to `"blocked — (no blocker
specified)"` so owner can still see block-state even when relation
is not captured. Surfaces the data-shape gap without hiding the
block-state itself.

## a11y

- `role="img"` on glyph, `aria-label="blocked"` with blocker text
- dim (opacity ~0.7) must still meet WCAG AA contrast on row body
  text — verify via phase-1a a11y tokens
- keyboard-focusable badge when `onOpenBlocker` handler provided
  (matches current `onOpenTimeline` pattern)
- screen-reader announces "blocked, waiting on issue #337 — open
  blocker" when badge focused

## interaction with filter / view

- Existing message-class filter (phase-1b/01) doesn't affect board
  rows — no change
- If Ryan adds a board-side filter in future ("show only actionable-
  for-me"), it would hide blocked rows — but that's an OPT-IN filter,
  not the default (per nothing-hidden principle)

## acceptance criteria (augmenting #390 filed ACs)

1. Story with `blocked` label in DELIVERED renders with distinct
   glyph + dim row + blocker badge.
2. Blocker rows stay visible in DELIVERED (not hidden / not collapsed
   into a secondary section by default).
3. Delivered count includes blocked rows (not excluded).
4. Tooltip/aria on badge surfaces blocker reference (or fallback text
   if label missing).
5. Click on blocker badge opens the blocker-referenced drill-in.
6. WCAG AA contrast on dim row (verify with a11y tokens).
7. Keyboard-accessible focus + activation on the blocker badge.
8. Playwright real-backend test: story with `blocked` label → block
   glyph + badge visible; story without → not visible. Verify with
   `blocked-by:N` label too (tooltip + click-nav).

## out of scope

- Structured `blocked-by` data model beyond labels (separate Feature;
  label-derived is the MVP).
- Auto-detecting blockers from story-body references (too
  speculative).
- Surfacing block-state on rows OUTSIDE the DELIVERED column (ready /
  icebox don't carry accept semantics; re-examine if owner pain
  surfaces there).
- Cascade-unblock (auto-remove block when blocker accepted) — future
  nice-to-have.

## desk-check walk script

Fixture setup (pre-walk): pm uses existing `label_story` ADD to seed
at least one DELIVERED story with `blocked` + one with `blocked-by:<N>`
labels. Mutation-roundtrip is OUT OF SCOPE for this story — it rides
the separate `label_story` completeness story (dev retro commit #2).
This story verifies rendering given labeled fixtures; not mid-session
mutation.

Designer walks (simulating Ryan):
1. Open DELIVERED column. Within 1s, can I see which rows are blocked
   vs awaiting-accept? — yes/no.
2. On a blocked row, is the blocker reference discoverable? (tooltip
   on hover, tab-focus + read aria-label with screen reader.)
3. Click blocker badge — does it open the right drill-in?
4. Verify dim-row is still readable in both themes (light + dark).
5. Verify count (7) includes the blocked rows.
6. Verify fixture-without-`blocked`-label renders no-badge (negative
   case). Mutation roundtrip (remove label → badge disappears) is
   deferred to `label_story` completeness work.

Walk pass = observation-falsifier doesn't fire at any step.
