# Cassiopeia iteration-container UX

*Designer — 2026-05-15. UX spec for the Cassiopeia substrate-pivot rework: iterations as collapsible containers + drag-within + drag-across-container + Unscheduled implicit container + per-story label-suffix-position-encoding.*

## Problem

PM (or Ryan) organizes the icebox by iteration-membership. Pegasus shipped iteration_id as a foreign-key label (correct substrate) but with explicit add/remove-ceremony tools (wrong UX). The first Cassiopeia attempt pivoted to positional-implicit-membership in a flat icebox (clever substrate-shape) but shipped:

1. **Render-source mismatch**: `apply_priority_to_board` reads legacy `priority.columns`, not the new `icebox_unified` — drag-reorders revert ~1s after drop (Ryan-empirical 2026-05-15 11:48 UTC).
2. **Storage-axis wrong shape**: order lives in local `priority.json`, not canonical-via-GH — instances disagree (Ryan-canon 2026-05-15 11:49 UTC).
3. **Visual-affordance half-delivered**: CSS orphaned from prior `<Iteration>` component; markers render as plain story-rows with a glyph-only signifier.

The rework adopts the substrate-shape Pegasus had right (labels on stories as canonical-via-GH membership signal) plus the UX-shape Pegasus had wrong (drag IS the ceremony; no explicit add/remove tools — Slice E's tool retire holds). And it returns iterations to first-class **containers** (collapsible `<details>` regions), not positional anchors in a flat list. This matches the visual Ryan referenced 2026-05-14 20:09 UTC ("I liked how it looked when you made it before") and is closer to the mental model users expect from Tracker/Jira/Trello: containers + explicit-belonging via drag.

## Direction (locked)

- **Iteration = collapsible container** (`<details>` element; sister to existing Epic.tsx pattern)
- **Membership + position = per-story label** — e.g. `iteration-Cassiopeia.0`, `iteration-Cassiopeia.1`, ... (name-anchored prefix + dot separator + dense-index position-suffix per Ryan's 2026-05-15 12:23 + 12:49 UTC framing). Dense-index is workable thanks to step 10 (outbox last-update-wins coalesce per (issue, label-class) — supersedes intermediate sequences so renumber-cascades coalesce in the drain window)
- **Iteration order itself = label on the iteration issue** — `iteration-order.0`, `iteration-order.1`, ... dense-index sister pattern
- **Unscheduled = implicit container** for loose stories; auto-created on first-boot
- **Drag IS the ceremony** — no explicit add/remove tools (Slice E retire holds; tools stay gone)
- **Canonical-via-GH** — labels are the substrate; no local `priority.json` ordering state
- **Cross-instance propagation** — existing TagAdded/TagRemoved + state-delta broadcast (no new infrastructure)

## Specs

- [Iteration container](iteration-container.md) — `<details>` shape + collapse-state persistence + drag-between-containers semantic
- [Unscheduled container](unscheduled-container.md) — implicit container for loose stories
- [Drag pattern](drag-pattern.md) — ChosenPicachu HideElemOnDragSortableListEagerMove + cross-container drag affordance
- [State-coherence UX](state-coherence.md) — optimistic-update + conflict-resolution + cross-instance state-delta visual

## Stories

- [#1297 Slice A](https://github.com/RyanDur/bpmux/issues/1297) — iteration-container shape (restore Iteration.tsx + Unscheduled auto-create + apply_priority_to_board label-parse)
- [#1339 Slice B](https://github.com/RyanDur/bpmux/issues/1339) — drag-within + drag-across container + drag-iteration-order
- [#1300 Slice C](https://github.com/RyanDur/bpmux/issues/1300) — close_iteration walks members via label-parse
- [#1301 Slice D](https://github.com/RyanDur/bpmux/issues/1301) — migration priority.json → labels + retire local substrate
- [#1299 Slice E](https://github.com/RyanDur/bpmux/issues/1299) — PASS (tools retired permanently; drag IS the ceremony)

All 4 reworked stories carry cross-instance + persist-across-refresh falsifier-axis ACs per Ryan-TDD-canon ("bugs aren't strategy; failing-test-first"). A 2nd AC-reshape pass aligns them with iteration-as-container direction.

## Source

- Ryan 2026-05-14 20:08 UTC: "marker data should be stored in github and not a json file"
- Ryan 2026-05-14 20:09 UTC: pre-Pegasus visual reference ("I liked how it looked when you made it before")
- Ryan 2026-05-15 11:48 UTC: empirical drag-doesn't-persist observation
- Ryan 2026-05-15 11:49 UTC: order-as-canonical-via-GH directive
- Ryan 2026-05-15 11:57 UTC: bugs-aren't-strategy + failing-test-first canon
- Ryan 2026-05-15 12:23 UTC: iteration-as-container + collapsible + per-story-tag reframe
- ChosenPicachu reference: https://github.com/RyanDur/ChosenPicachu — HideElemOnDragSortableListEagerMove sister-pattern
- Existing Epic.tsx (`applications/electron/src/renderer/src/board/Epic.tsx`) — `<details>` sister-pattern already deployed for Epics
