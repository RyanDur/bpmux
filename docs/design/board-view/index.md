# Board view — glanceable project state

*Designer — 2026-04-12. Pivotal-Tracker-inspired board showing backlog state at a glance.*

## Problem

Ryan has to ask "how's the backlog?" and wait for a pm status dump. the board should BE the answer — visible at a glance, no conversation needed.

## Domain model

- [Board & Backlog — two entities over one story lifecycle](board-backlog-model.md) — the settled entity boundary the board/backlog UI builds on (backlog = ledger of record; board = current-iteration projection into the detailed lanes).

## Discovery

- [Backlog view — the PM's needs](backlog-needs.md) — the PM's four jobs (author · triage · order/schedule · plan-with-owner), pains-forward, grounded against the `bpmux` substrate. North-star: backlog state is invisible-until-manually-read; make it glanceable. Input to the Ryan + designer design synthesis.

## Design-of-record (current)

- [**Board / Plan reorganization**](board-plan-reorganization-mockup.html) — Ryan-directed 2026-06-30. The split already exists in the shipped `[Now|Plan|Done]` tabs; this re-chromes it: top `[Board | Plan]`, Board = the current iteration, **Plan** (was Roadmap) → sub-views `[Roadmap | Backlog]`, Done tucked into Backlog. Re-chroming of shipped components — no new visual behavior. Awaits Ryan's react + PM's read.

## Mockups (historical — TUI-era, pre-Electron)

- [Board default view](board-default.md) — the glanceable state (2026-04-12, stale)
- [Board with details](board-details.md) — drill into a story (stale)

## Story

TBD — **no stories until planned** (Ryan 2026-06-30); pm files after mockup converges.
