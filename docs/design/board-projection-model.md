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
- **Beyond the board:** `accepted` — the terminal position reached by succeeding at
  acceptance. **`accepted` is not on the board.**
- Each state has a **judge** (a role/agent). Movement is a **verdict** the judge fires:
  - **Succeed** → the next state in order (deterministic — forward is just "next").
    At `acceptance`, success → `accepted` (off the board).
  - **NeedsWork** → the state's configured fail-target, or, if unconfigured, a place
    the judge chooses at fire-time (non-deterministic — "back to somewhere earlier").
- **One story moves at a time.** A verdict changes that one story's position field;
  every projection re-derives. No multi-story coordination.
- The gate / evidence / role-authorization machinery attaches to the verdict at a
  state (acceptance's judge = the story's acceptor).

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
- **Verdicts, not gates.** Two edge kinds per state (`Succeed` / `NeedsWork`)
  generate the machine; the seven-name `Gate` enum was a denormalized expansion.
- **`accepted` is off the board.** The board ends at `acceptance`; `accepted` lives
  only in the backlog `accepted` column. Same single field, different projection filter.
- **Three orderings, kept distinct:** priority (composition lists), flow sequence
  (the workflow), and none / soft drift (between the backlog's four columns).

## What this replaces in current code

- `BoardState` (7 rails) → one workflow-position id (`ready` / a middle stage /
  `acceptance` / `accepted`); `icebox` / `readyForWork` / `current` stop being states
  and become derived backlog columns.
- `ColumnBucket` + `column_for` → gone; the board is `Map<State, [StoryId]>` with no
  board-column type. (S2's `Middle(StageId)` + `header()` were still the old
  fixed-column thinking; `header()` was a view concern leaking into the domain.)
- `Gate` + transition table + `state_ordinal` / `is_backward` → verdicts over the
  config's states.
- `ready_order` + `is_closed` on the story → shed (order = list index; closed derived).
- The S2 wire-codec direction (open `as_str` / `parse`) is subsumed: the wire carries
  the story's one position; there is no rails-vs-middle split to "open."

## Status of prior slices

S0 (`StageId` seam), S1 (`InProgress(StageId)`), S2 (config-driven middle
arrangement) are landed on origin. They built the `StageId` identity this model
reuses as the workflow-position type, so they are not wasted — but
`ColumnBucket` / `column_for` / `header()` from S2 and the `Gate` / ordinal apparatus
are superseded and will be dismantled by this remodel. Sequencing the dismantle +
rebuild into gated slices is the next planning step (not in this doc).

## Open questions (unresolved with Ryan)

- **Epics** — a PM type today, not placed in `roadmap → outcome → iteration → story`.
  Likely an orthogonal cross-cut (an epic references stories across iterations), not a
  level in the composition — unconfirmed.
- **Retired / closed** — the backlog is four columns (icebox / readyForWork / current
  / accepted); no retired. How a closed / abandoned story is modeled (a fifth column?
  archived out? folded into accepted?) is unresolved.
