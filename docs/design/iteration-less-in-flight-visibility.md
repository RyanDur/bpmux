# Iteration-less in-flight work must be visible to the owner (#1757 design-shape)

> Design-of-record for #1757 [Bug] [Board] — in-flight story hidden from owner when
> it belongs to no iteration. PM filed (WHAT pinned, acceptor owner); designer shapes
> the owner-visible surface; dev owns the mechanism. Grounded firsthand against the
> render path, not sketched — the gap + the existing substrate to build on are both real.

## The principle (Ryan-canon 2026-06-20)

Iteration-membership decides WHICH LANE a story sits in, **never WHETHER the owner
sees it.** Every in-flight story (in-progress / desk-check / delivered) is visible on
the owner board regardless of iteration-membership or acceptor-class. Visibility and
accept-authority are different axes — a chore being dev's to ACCEPT does not make it
hidden from the owner. This is the nothing-hidden-from-owner principle (#385 lineage:
"Channels hide messages from Ryan"). The cure is a surface that SHOWS, never a filter
that hides.

## The gap, grounded (the exact render locus)

`board/BoardTabs.tsx` `renderNow()` renders the in-flight workflow-state columns
(NOW_STATES = `ready · in-progress · desk-check · delivered`) but filters each to the
current iteration's members:

```
groupsFor(NOW_STATES).map((group) =>
  renderGroup(group, undefined,
    (grouped.get(group.state) ?? []).filter((s) =>
      currentIterationMemberNumbers.has(s.number))))   // <-- the membership filter
```

`currentIterationMemberNumbers` derives from `deriveNowView` = the is_current
iteration's members (Carina #1549). A story in an in-flight state but a member of NO
iteration (e.g. #1755 — a desk-check chore at iteration:None) IS in
`grouped.get('desk-check')` but NOT in `currentIterationMemberNumbers`, so the filter
drops it. It surfaces nowhere else:
- **Now** tab: filtered out (above).
- **Plan** tab: renders Epics + icebox/iteration-containers only; an in-flight
  (non-icebox) story isn't there.
- **Done** tab: renders `accepted · retired` — and notably with NO member filter
  (`renderGroup(group)`), so Done already shows everything. In-flight ≠ done, so not there.

So iteration-less **in-flight** work is the single class that vanishes. (Done already
honors nothing-hidden; only the Now-tab in-flight lanes apply the membership filter.)
This is a STANDING gap (the WS-migration changed transport, not this filter), not a
recent regression.

## Existing substrate to build on

`board/UnscheduledList.tsx` ALREADY implements the "Unscheduled (N)" collapsible
`<details>` for iteration-less stories — story rows with StoryTypeGlyph + StoryBadge +
navigate. But it's wired only into `Iteration.tsx` (the **Plan** tab's planning/icebox
context). The concept + component exist; the **Now** (in-flight) tab just has no
equivalent surface. So this is an extension of an existing pattern, not a new vocabulary.

## Recommended shape — an "Unscheduled" in-flight section in the Now tab

In `renderNow()`, after the current-iteration columns, add a secondary section that
renders the same NOW_STATES groups filtered to the COMPLEMENT — in-flight stories
NOT in `currentIterationMemberNumbers` — reusing the Unscheduled disclosure grammar
(collapsible, count, StoryBadge). Why this shape:

- **Preserves the Now tab's primary job** — "how is the active iteration progressing,
  at a glance." The orphans sit in a secondary, collapsible section beneath the primary
  lanes, so they never clutter or displace the current-iteration read.
- **Satisfies WHICH-LANE-not-WHETHER exactly:** members → the primary lanes; orphans →
  the Unscheduled lane; everyone visible. Iteration decides placement, not visibility.
- **Maps to existing structure** — a sibling section in `renderNow`, a complement-filter
  of the same `grouped` columns, reusing UnscheduledList's grammar. Small, coherent.
- **Zero cost in the common case** — UnscheduledList returns null on empty, so when
  there are no orphaned in-flight stories (the norm) the section renders nothing.

## Alternative considered (and why secondary)

**Drop the member filter — show ALL in-flight in the primary columns, annotate
non-members.** More visibility-first, but it changes the Now tab's meaning (today it's
scoped to the active iteration's progress) and risks clutter when orphans are many. The
owner's primary Now-job is the active iteration; orphans are the exception — a secondary
section serves full visibility without diluting the primary read. So: rejected as the
default, available as a fallback if the secondary-section read tests as too buried.

## Observation falsifier

Reject if: an in-flight story (in-progress / desk-check / delivered) with no
current-iteration membership appears NOWHERE on the owner board; or the fix is a
renderer filter that hides rather than a surface that shows; or surfacing orphans
clutters/displaces the current-iteration primary lanes; or the Unscheduled-in-flight
section renders a non-empty shell when there are zero orphans.

## Scope note

Mechanism is dev's (the complement-filter + section wiring; whether the orphan-set
derives in `useBoardData` or inline in `renderNow`). This doc pins the owner-visible
SURFACE + the principle it must satisfy. PM places #1757 at the planning beat
(Circinus-candidate, theme-coherent with the board-honesty / agent-affordance work).
