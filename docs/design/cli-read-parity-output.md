# CLI read-parity output — the roadmap-view as the agent reads it

> Design-of-record for the read-parity Feature (agent-affordance theme).
> Output-SHAPE = designer's lane; verb-MECHANISM (substrate-read) = dev's.
> Text-twin of the GUI: deriveRoadmapView (overview) + deriveOutcomeDrillIn (drill).
> Grounded firsthand against both derivers — mirror them EXACTLY, never over-render.

## Verb-shape (settled with PM)

Repurpose `bpmux roadmap` -> print the substrate-VIEW by default (parity); demote the
markdown narrative to `--narrative`. NOT a new verb: a new verb leaves `roadmap` printing
the retired markdown (the trap). Repurposing makes parity the DEFAULT and retires the
markdown-as-default for the CLI exactly as #1704 did for the GUI. Same retirement, both surfaces.

## Token-frugal by design (Ryan-canon 2026-06-16: "only enough to look up details · don't drown anyone / take all their tokens")

THE governing constraint — applies to EVERY agent-CLI read, not just this verb. Return the
MINIMUM to LOOK UP + a cheap drill-path; never a firehose. Concretely:
- The OVERVIEW row is the look-up INDEX: `#number  name  <state>` — the keys an agent needs to
  decide + drill. The why / convene-checks / per-iteration detail are NOT in the overview; they
  live in the DRILL (`--outcome <N>`), pulled only when needed. The DISCLOSURE CHAIN is three
  levels, each pulled on demand: `bpmux roadmap` (lean overview index) → `--outcome <N>` (one
  outcome's iterations + checks) → `bpmux story <N>` (a story's full body, the existing verb).
  Same bar board / ready / story already hold — never a flooding dump.
- `--state` / `--limit` narrow BEFORE rendering (fetch-what-you-asked, not filter-a-dump), so a
  scoped read costs scoped tokens. `--limit` labels what it elided (no silent-truncation).
- The drill is one outcome's detail, not all. No verb dumps the whole substrate by default.
This is the appropriate-amount discipline ([[feedback_appropriate_amount_right_sized_feeds_over_firehose]]):
the minimum that answers the question + a cheap path to more — token-frugal, by design.

## What Ryan needs to see (owner-spec 2026-06-16)

1. what OUTCOMES there are + their PRIORITY → the overview bands.
2. what ITERATIONS are in an outcome + their PRIORITY-ORDER → the drill.
3. whether each is DONE / READY / NEEDS-ATTENTION → the state vocabulary (below).

## OVERVIEW (default `bpmux roadmap`) — mirrors deriveRoadmapView

Bands by horizon = PRIORITY (Now / Next / Later); each outcome ONE scannable, grep-able row =
`#number  name  <readiness>`. readinessState (from the substrate):
- `none` (iteration_count === 0) -> **planned · iterations TBD**  (honest empty-outcome; NOT a 0/done read)
- `gap`  (has iterations, not all ready) -> **N needs-attention** (gapCount)
- `ready` (all ready) -> **ready**

```
ROADMAP
Now    #12  Decisions stop getting re-litigated     ready
Next   #34  Decision-layer                          planned · iterations TBD
       #35  Darcula                                 planned · iterations TBD
Later  #36  Ship without flaky-gate friction        planned · iterations TBD
```

## DRILL (`bpmux roadmap --outcome <N>`) — mirrors deriveOutcomeDrillIn EXACTLY

Per-iteration IN PRIORITY-ORDER (`order` 1..n), each = `#number name [state] (gapCount)` + the 4
convene-checks. State (DrillIterationState — readiness-to-convene): `ready` | `next` (the active
gap-holder) | `gap` | `not-started`. The 4 checks (CHECK_ORDER) each `met` | `gap` | `pending`:
drafted / stories-ordered / designer-review / dev-OK. Plus the convene-banner when a next exists.

```
bpmux roadmap --outcome 34 :
#34 Decision-layer
  convene? next: Crux-1740 — 2 gaps · then Crux-1741
  iterations (priority order):
    1. #1740 Crux-1740   [next]         2 gaps
         drafted met · stories-ordered GAP · designer-review pending · dev-OK pending
    2. #1741 Crux-1741   [not-started]
```

**Do NOT over-render** (PM caught this in the first sketch): there is NO `[active]` state and NO
`8/10 accepted` count — accepted-count is EXECUTION-progress (a SEPARATE, unbuilt work-progress
axis = the roadmap Seeds indicator), NOT in the readiness substrate. Promising it = the CLI
over-renders vs the GUI = not parity. Mirror OutcomeDrillBody/deriveOutcomeDrillIn exactly.

## State vocabulary — Ryan's done/ready/needs-attention mapped to the substrate

- **ready** = `ready` (all 4 convene-checks met).
- **needs-attention** = `gap` or `next` (has unmet checks / is the active gap-holder).
- (overview) **planned** = `none` (no iterations yet).
- `not-started` = a building iteration with nothing done yet (distinct from `none` = no iterations).
- **DONE (RESOLVED — INCLUDED, Ryan + PM scope-call 2026-06-16):** Ryan named it ("done / ready
  / needs-attention"), so v1 is NOT strict-deriver-parity — the read surfaces the DONE axis
  (is_closed / shipped) ALONGSIDE readiness. At the DRILL iteration-level each iteration reads:
  `done` (shipped/closed) | `ready` (convene-ready, all checks met) | `needs-attention` (next/gap)
  | `not-started`. Rolled to the OVERVIEW outcome (e.g. all-iterations-done → outcome reads done).
  The current deriver (deriveOutcomeDrillIn) has readiness but NOT done → the verb ADDS the
  is_closed surface (dev's mechanism, beyond the deriver). GUI<->CLI: for full parity the GUI
  could later surface done too (designer's lane, separate from this).

## Empty-state note (CLI improves on the GUI)

`planned · iterations TBD` is honest AND clearer than today's GUI (the GUI suppresses the `none`
badge -> renders nothing; the empty-outcome drill shows the build-up title + an empty list). So
the CLI improves on the GUI here. For full GUI<->CLI parity the GUI could later adopt
`planned · iterations TBD` too — a small render-call, designer's lane, separate from the fill
(which renders safe either way).

## Query affordances — filter / limit / order (Ryan 2026-06-16)

Any view (overview or drill) is filterable + limitable + sortable, composably, per clig.dev's
human+agent-friendly list grammar:
- `--state <state>` — filter to a state: overview `ready | needs-attention | planned`; drill
  `ready | next | gap | not-started` (+ `done` if the done-axis is added). The acute one:
  `bpmux roadmap --state needs-attention` = "show me only what needs my attention."
- `--limit <N>` — cap the rows (top-N).
- `--order <field>` — sort key. DEFAULT = priority (the substrate band/position order Ryan sees,
  so unflagged output IS owner-parity); other keys: `gap-count`, `state`, `name`.

They COMPOSE: `bpmux roadmap --state needs-attention --order gap-count --limit 5`. Filtering by
state is the same honest substrate vocabulary as the unfiltered rows — no new states invented.
A filtered/limited view still LABELS what it elided (clig.dev no-silent-truncation: e.g. "showing
5 of 12") so a `--limit` never reads as "that's all there is."

## Parity check — Ryan's 3 + PM's 4 verify-not-write-blind scenarios all served

Outcomes+priority (overview bands) · iterations+priority-order (drill `order`) · done/ready/
needs-attention (state vocab) — and: verify-my-write (read the outcome's row + drill after
create/add) · check-readiness (overview states + gap-counts + convene-banner) · owner-parity
(same Now/Next/Later bands Ryan sees) · empty-outcome (`planned · iterations TBD`).
