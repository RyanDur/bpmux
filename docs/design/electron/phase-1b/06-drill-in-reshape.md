# phase-1b drill-in visual-hierarchy reshape

Ryan 2026-04-23 16:41-16:43 UTC during acceptance walk on #402 drill-in
(img_18): *"is this supposed to be a dropdown picker?"* → *"I don't
understand what it is trying to convey"* → *"visually it is not well
organized to inform me."* Subsequent reject on #370 (avatar tooltip)
+ rejects on #366/#367/#371 cited the drill-in UX as the blocker.

Ryan 2026-04-23 23:53 UTC: *"i won't accept story based stories
because the view is so bad, that i cannot understand what i am
looking at. it seems like the designer did not look at these."*

Current drill-in renders state/labels/assignees as plain stacked text;
lifecycle-tracking labels (delivered, ready-for-accept, desk-check-
fail, blocked) leak substrate into the user surface; empty sections
render "(none)" as noise; chip-styling from `DrillIn.css:drill-in-
chip` fails to apply (rendering as plain text, not pill-chips).

Four failure-classes stack on one surface: implementation-detail leak
+ visual-hierarchy flat + empty-section-noise + chip-rendering bug.
Reshape addresses all four as a single coherent spec.

## 3-field spec header

**user-job:** When Ryan (or any role-user) opens a story drill-in, the
meta-section informs them at-a-glance of the story's current situation
— WITHOUT requiring interpretation of GitHub-label machinery, and
WITHOUT consuming scroll space for empty-field noise. The drill-in IS
the focused-read surface; everything above the body should earn its
real-estate by informing.

**observation-falsifier:** I would reject if:
- the drill-in renders lifecycle-tracking labels (delivered,
  ready-for-accept, desk-check-fail, blocked) as user-visible entries
  — these are substrate-state, not user-meaningful labels
- sections render "(none)" when empty — empty = no information, hide
  the row
- state + labels appear as plain-stacked text instead of visually-
  distinct affordances
- the user cannot identify story's lifecycle-situation (delivered /
  rework / accepted / blocked) within ~1s of opening the drill-in

**team-shape-independence:** lifecycle-label filter uses
message-type-based detection (same pattern as `deriveTeamRoles` + state-
derivation); no hardcoded team-role references.

## principle alignments

- **implementation-detail leak in UX surface** (2026-04-24 designer
  lesson): lifecycle labels are bpmux-substrate; filtering them out of
  user surface is the lesson's direct application. Users care about
  situation; substrate cares about state-machine tracking.
- **nothing-hidden-from-owner**: lifecycle labels hidden from meta-list
  doesn't hide STATE — state surfaces via the hero state-badge.
  Filtering is semantic-clarification, not information-removal.
- **data-ink audit** (Tufte): every pixel on the drill-in surface
  should carry user-information. "(none)" rows carry zero; remove.
- **substrate-consistency**: reuse StoryBadge (#368/#375/#376/#390)
  for the hero state — same visual vocabulary as board rows. Users
  already learned the badge-language there.

## visual treatment (behavioral, not implementation-prescriptive)

### hero state-badge

At the top of the drill-in meta-section (between title + body), render
a large, visually-prominent state-badge using the existing
`StoryBadge` component from the board. Same vocabulary + colors + icon
as board rows:
- rework (red): story in desk-check-fail cycle
- blocked (amber + ⛔): story blocked per #390
- team-accepted (solid green): story in bug/chore lane
- owner-awaiting / owner-accepted: story in feature lane
- awaiting-your-accept vs in-rework distinguished per #375/#376

The hero-badge answers Ryan's #1 scan question: "what's happening
with this story right now?" at glance-speed without reading text.

### filter lifecycle labels from the labels-row

Labels that are state-tracking substrate do NOT render in the drill-
in labels-row:
- `delivered`
- `ready for accept`
- `desk-check-fail`
- `blocked`
- `blocked-by:<N>` (but DO render the blocker-reference in the hero
  state-badge tooltip per #390)
- `rework`
- `rejected`

These are lifecycle machinery, not user-meaningful labels. The hero-
badge + state (delivered/accepted/etc columns) already carry this
information semantically.

Non-lifecycle labels DO render as chips (if any exist). Common
non-lifecycle labels: category-tags, topic-tags, ad-hoc classifiers.

### hide empty sections

- If labels-row (post-filter) is empty → hide the labels row entirely.
- If assignees array is empty → hide the assignees row entirely.
- If future meta-fields land empty, hide them as well.

Empty-section-noise costs scroll real-estate and conveys nothing.
"(none)" is implementation-detail artifact; user-lens has no
information-value in rendering it.

### fix chip-rendering bug

Current `DrillIn.css:drill-in-chip` has `padding + border + border-
radius: 12px + background`. Ryan's live render shows plain stacked
text → CSS cascade is failing (nested selector parse, specificity
conflict, or stale build).

Diagnose + fix. Verify via Playwright that `.drill-in-chip` computed
style has the border + border-radius + background applied.

### layout structure (behavioral, not impl)

```
┌─ drill-in ────────────────────────────────────┐
│ ← Back to board                                │
├────────────────────────────────────────────────┤
│ #402 · Title of the story                      │
│                                                │
│ ┌──────────────────────────┐                   │
│ │ ⛔ blocked — waiting #337│ ← hero badge      │
│ └──────────────────────────┘                   │
│                                                │
│ [ bug ] [ discovery ]                ← chips   │
│ (labels row hidden if empty)                   │
│                                                │
│ Story body text here…                          │
│ (rendered markdown)                            │
│                                                │
│ Comments section                               │
└────────────────────────────────────────────────┘
```

- No more "state: delivered / labels: delivered, ready for accept /
  assignees: (none)" dl-style meta-stack
- Hero badge = primary signal
- Non-lifecycle chips = secondary classification (if any)
- Body = content
- Comments = content

## acceptance criteria (behavioral)

1. Drill-in renders hero state-badge at top (below title, above body)
   using StoryBadge component; badge color + shape matches board row
   for same story.
2. Lifecycle labels (`delivered`, `ready for accept`, `desk-check-
   fail`, `blocked`, `blocked-by:<N>`, `rework`, `rejected`) are NOT
   visible in the drill-in meta-list.
3. Non-lifecycle labels render as chip-pills (with border + border-
   radius + visible padding), not plain stacked text.
4. Empty labels-row (post-filter) is hidden (not rendered as "(none)").
5. Empty assignees-row is hidden.
6. Meta-stack's `<dl>` + `<dt>state</dt><dd>{state}</dd>` pattern is
   retired (state now lives in hero badge).
7. Ryan (or any user) can identify the story's lifecycle-situation
   within ~1s of opening drill-in.
8. Playwright real-backend test: open a story with lifecycle labels,
   assert none of them render in the visible meta-list; hero badge
   renders with correct state.

## out of scope

- Rework of comment-section styling (separate concern from meta-list)
- Rework of timeline section (already rendered per #367 — which
  depends on this reshape shipping first)
- Assignees-as-@mention-pill (if/when non-empty assignees become a
  thing; not today's pain)
- Drill-in sidebar / related-stories panel (future feature-layer)

## desk-check walk script (designer user-story walk)

Fixture: a story in delivered state with `delivered`, `ready for
accept`, and one non-lifecycle label (e.g., `bug`).

1. Open drill-in. Within 1s, can I identify the lifecycle situation?
   Expected: hero badge shows "awaiting owner" or equivalent.
2. Scroll through meta-section. Does it contain "delivered" as a
   label? Expected: NO (filtered).
3. Does it contain "(none)" anywhere? Expected: NO (hidden).
4. Is the non-lifecycle `bug` label rendered as a chip-pill? Expected:
   YES, with border + padding + visible shape.
5. Open a story with no assignees. Is there an assignees row?
   Expected: NO.
6. Open a blocked story (`blocked` + `blocked-by:337`). Does hero
   badge show the blocker reference? Expected: YES per #390 badge
   behavior.

Walk pass = observation-falsifier doesn't fire at any step +
positive scan-test passes.

## dependencies

- #390 (blocker visual) already delivered + accepted — StoryBadge has
  blocked variant; reshape reuses it.
- #368/#375/#376 (state-aware badges) — StoryBadge substrate exists.
- Chip-rendering-failure investigation (dev-lane) — may be standalone
  small fix or folded into this reshape impl.

## dependency-order implication

**Ships BEFORE:**
- #366 Comment quote-excerpt (Cmd+E in drill-in)
- #367 Timeline audit-trail (renders inside drill-in)
- #371 Right-click context menu (drill-in-scoped)
- Any future drill-in feature

Per designer retro-commit on backlog-ordering-by-dependency: visual-
surface-features stack on their surface. Ship the surface first.

## meta: why this failed initial shipping

From retro 2026-04-24: today's 4 drill-in rejects (#366/#367/#371 +
#370 tooltip-thin) all hit the SAME underlying cause — drill-in meta
rendered raw substrate + chip-rendering-bug + empty-section-noise.
Each reject was treated as story-specific. Meta-pattern not caught
until Ryan's "designer did not look at these."

Designer-apply-gap: I named drill-in concern at 13:25 UTC (#337 PASS
note) + 16:41 UTC (img_18 arc) but tone was "follow-up item," not
"urgent escalate before new drill-in features stack." Match-urgency-
tone-to-impact discipline applied forward.
