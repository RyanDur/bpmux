# The board–backlog–roadmap projection model (#69)

Status: converged with Ryan 2026-07-02 through a full model walk-through. This is
the authoritative design and it **supersedes** the earlier `column_for` /
`ColumnBucket` / `Gate` / wire-codec direction — including the S0–S2 slices built
toward it (see "What this replaces" and "Status of prior slices").

## North star (must survive compaction)

> A story stores exactly **one** fact: its position in the workflow.
> The reference **lists** store order.
> The **workflow** (`.bpmux/workflow.toml`) is the guard rails that move a story's position.
> The **board**, the **backlog**, and the **roadmap** are all **projections** of those two facts — they store nothing.

**This remodel is a REFACTOR (Ryan, 2026-07-04).** No behavioral changes apart
from any performance gains; the app works as-is throughout. The deliverable is
organization — code that prompts future readers into better decisions: more
readable, less unnecessary context when a file is opened, concerns homed in
their proper domains, better patterns. The visible surface and the wire do not
change under this arc; any deliberate wire or surface reshape is a separate,
owner-approved arc, never a side-effect of a collapse slice.

## Two sources of truth — and nothing else is stored

1. **A story's workflow position** — one field on the story.
2. **Order** — held by the reference lists, as index (top = highest priority).

`is_closed`, which board column, which backlog column, `ready_order` — all derived.

## Entities — the composition (priority-ordered reference lists)

```
Roadmap   → [OutcomeRef]     (priority order)
Outcome   → [IterationRef]   (priority order — the iterations that achieve it)
Iteration → [StoryRef]       (priority order)
Story     → leaf: content + one field (workflow position)
```

Every level is the same shape: an ordered list of refs to the level below, ordered
by priority. Order = index in the containing list. Persisted at the edge as the
list's ref order. This kills `ready_order` on the story — a story's rank *is* its
index in whatever list references it.

## The workflow — the guard rails, from `workflow.toml`

The workflow and the board are **separate**. The workflow defines *which states
exist* and *which moves are legal*; the board is those states *populated* with the
current iteration's references. Name it "workflow" (it already is — `WorkflowConfig`
/ `workflow.toml`), never "machine."

- **Board states:** `ready` (stable start) → `[middle]` (config-defined, N stages) →
  `acceptance` (stable end). The board is exactly these. Only the middle comes from
  the toml; `ready`/`acceptance` are fixed bookends.
- **Beyond the board (terminals, both off-board):** `accepted` — reached by
  succeeding at acceptance (the "done / succeeded" terminal); `retired` — reached by
  abandoning/removing a story from anywhere (the "won't-do / superseded" terminal).
  **Neither is on the board.** "Closed" is GitHub's word for these — `is_closed` is
  the edge representation of `position ∈ {accepted, retired}`, not a domain fact.
- Each state has a **judge** (a role/agent). Movement is a **verdict** the judge fires
  — the judgment that the work meets (or misses) the team's quality bar. The verdict
  IS the gate; it drives the transition directly:
  - **Succeed** (meets the bar) → the next state in order (deterministic — forward is
    just "next"). At `acceptance`, success → `accepted` (off the board).
  - **NeedsWork** (misses the bar) → the state's configured fail-target, or, if
    unconfigured, a place the judge chooses at fire-time (non-deterministic — "back to
    somewhere earlier").
- **The verdict already exists.** `workflow/verdicts.rs` today stores an agent's
  grounding verdict as *evidence*, which a *separate* gate action (pass-desk-check /
  accept) consumes within a freshness window. This remodel collapses the two: the
  verdict fires the transition first-class instead of being posted-then-consumed. The
  named `Gate`s (deliver/pass/fail/reject/accept) each become "a verdict at a state";
  pickup (entering the board) is the one non-verdict move.
- **One story moves at a time.** A verdict changes that one story's position field;
  every projection re-derives. No multi-story coordination.
- The evidence / role-authorization machinery survives as the verdict's preconditions
  (acceptance's judge = the story's acceptor).

Deleted by this model: the `Gate` enum, the hand-written transition table, and
ordinal `is_backward` / `state_ordinal`. "Backward" is simply a `NeedsWork` verdict —
read off the move, never computed. Forward edges are generated from the state order,
never enumerated. Two edge kinds per state generate the whole machine.

## Projections — derived, own nothing

- **Board** = group the current iteration's stories by workflow position, filtered to
  `ready…acceptance` → `Map<State, [StoryId]>`; each bucket ordered by iteration
  index. Computed on read. `accepted` is excluded (off the board).
- **Backlog** = four columns, **no strict order between them** (soft drift), derived
  from (position + membership):
  - `accepted` → **accepted**
  - in the current iteration (position `ready…acceptance`) → **current** (this set *is* the board)
  - referenced by any iteration → **readyForWork**
  - else → **icebox**
- **Roadmap** = the composition tree (outcomes → iterations → stories), in priority order.
- **Views** = board-as-columns / backlog-as-lists / roadmap-tree / CLI / MCP. All
  visual concerns — headers, colors, display arrangement — live *here*, never in the
  domain. The domain hands over `State → [StoryId]`; the view decides it renders as a
  column titled "IN PROGRESS", a list row, or nothing.

## Persistence — the edge

A story is a GitHub issue; its workflow position is a label on the issue — decoded
per-story at ingest onto the story, re-encoded per-story at export from it. List
order = the ref order at the edge. Board, backlog, and roadmap are never persisted —
always projected. Consistent with the #58–#68 "GitHub to the edge" work.

## Key decisions (so they are not re-litigated)

- **State on the story, not in the board (model B, not A).** The board is a
  projection, not a store. A story persists as an issue with a state label, so the
  fact decodes/encodes per-story — the story is its natural home. Making the board a
  stateful aggregate would duplicate the label and fight per-issue persistence. "One
  story moves at a time" makes B safe: change one field, re-derive.
- **Order on the lists, not the story.** Rank is list index. `ready_order` dies.
- **Verdicts ARE the gates.** A verdict is the agent's judgment that the work meets
  the team's quality bar; that judgment fires the transition (Succeed / NeedsWork).
  This unifies today's split — a verdict posted as evidence + a separate gate action
  that consumes it — into one first-class move. The seven-name `Gate` enum was a
  denormalized expansion of "a verdict at a state."
- **`accepted` is off the board.** The board ends at `acceptance`; `accepted` lives
  only in the backlog `accepted` column. Same single field, different projection filter.
- **Three orderings, kept distinct:** priority (composition lists), flow sequence
  (the workflow), and none / soft drift (between the backlog's four columns).

## What this replaces in current code

- `BoardState` (7 rails) → one workflow-position id (`ready` / a middle stage /
  `acceptance` / `accepted` / `retired`); `icebox` / `readyForWork` / `current` stop
  being states and become derived backlog columns. `accepted` and `retired` are
  off-board terminals.
- `ColumnBucket` + `column_for` → gone; the board is `Map<State, [StoryId]>` with no
  board-column type. (S2's `Middle(StageId)` + `header()` were still the old
  fixed-column thinking; `header()` was a view concern leaking into the domain.)
- `Gate` + transition table + `state_ordinal` / `is_backward` → verdicts over the
  config's states.
- `ready_order` on the story → shed (order = list index).
- `is_closed` on the story → shed; it becomes edge-derived (`position ∈ {accepted,
  retired}`). The ~20 sites that branch on the bool re-express as "branch on position."
- The S2 wire-codec direction (open `as_str` / `parse`) is subsumed: the wire carries
  the story's one position; there is no rails-vs-middle split to "open."

## Status of prior slices

S0 (`StageId` seam), S1 (`InProgress(StageId)`), S2 (config-driven middle
arrangement) are landed on origin. They built the `StageId` identity this model
reuses as the workflow-position type, so they are not wasted — but
`ColumnBucket` / `column_for` / `header()` from S2 and the `Gate` / ordinal apparatus
are superseded and will be dismantled by this remodel. Sequencing the dismantle +
rebuild into gated slices is the next planning step (not in this doc).

## Resolved with Ryan (2026-07-03)

- **Epics** — not an open question; they have their place. An epic is an **orthogonal
  cross-cut** — it references stories across iterations, a grouping *over* the
  composition, not a rung on the roadmap → outcome → iteration → story ladder. This
  remodel does not touch them.
- **Retired / closed** — `retired` is an off-board terminal position (the
  won't-do / superseded sibling of `accepted`'s succeeded). "Closed" is GitHub's word
  for a terminal, so `is_closed` is **edge-only** (github-closed iff `position ∈
  {accepted, retired}`) and sheds from the domain. CONFIRMED (Ryan, 2026-07-04):
  a retired story is **archived** — queryable, absent from the active board and the
  four backlog columns, not rendered ("archived work that I don't need to see").
  The backlog's four visible columns, in Ryan's words: `icebox` = unplanned work;
  `readyForWork` = planned work, in an iteration on the roadmap; `current` = what
  work is on the board; `accepted` = completed work.
