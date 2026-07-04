# board-as-slice reshape — labels off the wire, containers own their children

Status: converged with Ryan 2026-07-04. Successor to `board-evaporation.md`
(Phase 0, wire-preserving relocation — runs FIRST) and sibling to
`board-projection-model.md` (the Phase-1 verdict collapse — this reshape and that
collapse overlap and should be sequenced together). This doc records the target
model so the convergence survives compaction; it is a design, not yet a slice plan.

## North star (must survive compaction)

> A "label" is GitHub's storage encoding — nothing more. It exists at exactly ONE
> seam: the GitHub adapter (`integrations/github`), which encodes typed domain →
> labels on push and decodes labels → typed on ingest. Nowhere else in the system —
> not the store, not the wire, not the renderer — does the word "label" appear.
> The wire speaks TYPED FACETS. Containers own membership and order. A story holds
> its intrinsic attributes and its relations-with-affordances; everything derivable
> is a selector, never a stored field.

## Why (the evidence that forced this)

- The GitHub label codec is implemented TWICE: `GhLabelCodec` (backend) and a
  mirror parser in the renderer (`board/labelSchema.ts`, `storyTypeFromLabels`,
  `blockerIdFromLabels`, `isLifecycleLabel`, `deriveBadge` over label strings).
  Labels ride the wire, so the frontend re-decodes GitHub's persistence format in
  the UI — the vendor encoding on the maximally wrong side of the adapter.
- `apply_labels_refreshed` stamps six decoded label-shadows onto the story
  (`acceptor`, `parent_epic`, `tags`, `blocked_by`, `ready_order`,
  `iteration_slot`) — so the store can represent ILLEGAL states: a `blocked` tag
  with no blocker, an `iteration_slot` disagreeing with the iteration's own
  `stories` list, a `ready_order` fighting the container. Deriving makes those
  states unrepresentable (designing-with-types).
- `sort_board_ready_by_membership` ALREADY derives ready order from container
  membership — the per-story `ready_order` is a pure shadow, proven by existing code.

## The facet map (each label-derived story field: stay or go)

| Facet | Verdict | Reason / where truth lives |
|---|---|---|
| `story_type` | **STAY** (typed on wire) | intrinsic attribute |
| `acceptor` | **STAY** (typed on wire) | intrinsic attribute |
| `parent_epic` | **GO** | the epic owns its children (see epics below); the story up-pointer is GitHub's child-label shape mirrored into the domain |
| `iteration_slot` | **GO** | `iteration.stories` (ordered) is the container truth |
| `ready_order` | **GO** | container ordering; already derivable (`sort_board_ready_by_membership`) |
| `Tag::Blocked` | **GONE** (evaporation slice 2) | unreachable + degenerate vestige |
| `blocked_by` | **GONE** (evaporation slice 2) | unfinished mechanism removed; the board story for blockers builds it right later (first-class block action + relation + glyph, no labels) |
| `Tag::DeskCheckFail` | **GO — it is an ACTION** | a `NeedsWork` verdict in the gate history; "did it fail desk-check" is a selector over the log. Folds into the Phase-1 verdict collapse (`board-projection-model.md`) — remove THERE, not piecemeal |

Post-reshape `Story`: `id, title, state, story_type, acceptor, body, filed_at,
filed_by, comments/gate-history, is_closed` (+ etag mechanics). No tags field, no
membership/order fields, no label shadows, no `raw_labels`.

## Epics are unfinished — parents hold their children refs (Ryan, 2026-07-04)

`Epic` (`epic.rs`) stores no children; `derive_epic_state`/`derive_epic_counts`
compute everything from `&[&Story]` gathered by REVERSE-lookup on each story's
`parent_epic`. That up-pointer is GitHub's shape (issues have no children list, so
parentage hangs on the child as a label) mirrored into the domain. Part of this
refactor is finishing the model: **the epic gains an ordered children ref list,
symmetric with `iteration.stories`**, and `parent_epic` leaves the story. The
GitHub adapter maintains the child-label representation at the seam (encode:
children list → per-child labels; decode: labels → children list).

## What the reshape deletes

- The wire `labels: string[]` on board stories (typed facets replace it).
- `Story.raw_labels` shadow + `apply_labels_refreshed`'s shadow-stamping.
- `StoryView::wire_labels` (re-encoding typed fields back into label strings).
- The ENTIRE renderer label parser (`labelSchema.ts`, `storyTypeFromLabels`,
  `blockerIdFromLabels`, `isLifecycleLabel`, label-based `deriveBadge` paths).
- The second codec: encode/decode exists once, in `integrations/github`.

## Coordination class

UX-INVARIANT and dev-owned end-to-end: the renderer draws the same cards, sourced
from typed fields + container order instead of parsed label strings. Same pixels,
different source — no designer gate for the mechanics. It IS a wire reshape
(labels out, typed facets in), which is exactly why it is sequenced AFTER the
wire-preserving evaporation and beside the Phase-1 verdict collapse, not folded
into Phase 0.

## Sequencing

1. `board-evaporation.md` completes first (wire-preserving; includes the
   `blocked_by` subtraction as its slice 2).
2. This reshape then runs WITH the Phase-1 verdict collapse — `DeskCheckFail`
   dissolves into the verdict model, the write path is already moving, and the
   epic-children + container-ordering moves land as part of board-becomes-a-slice
   (the redux-slice endgame: denormalized fields → selectors, one store, board as
   pure projection).
3. Slice decomposition happens when this arc opens, informed by the evaporation's
   residue — this doc pins the MODEL, not the slices.

## Key decisions (so they are not re-litigated)

- **Labels are adapter-internal.** One codec, at `integrations/github`. The wire is
  typed. The renderer never parses a label.
- **Containers own membership and order** (iteration → stories, epic → children,
  outcome → iterations — the last already true via `outcome.iterations`).
- **Derivable ≠ storable.** Status flags derivable from relations/history are
  selectors. The store cannot represent a disagreement between a flag and its source.
- **`blocked_by` was removed, not rejected** — the blockers story rebuilds the
  capability first-class when picked up. Removing the vestige ≠ deciding against
  the feature.
- **`DeskCheckFail` exits via the verdict collapse**, not piecemeal — it is the
  same concept as `NeedsWork(target)` in `board-projection-model.md`.
