# Spike 1: ingest-all latency benchmark

**Question**: how long does "ingest all stories + their structured comments" take against the live bpmux GH repo, at GH-authenticated rate?

**Why it matters**: spec's 3s target for candidate A (eager-mirror) hinges on this. If ingest is <1s, eager-mirror is cheap enough to make candidate A/D the clear pick. If 10s+, lazy-load (B) is load-bearing.

## Method

Ran from the bpmux repo root with GH_TOKEN + BPMUX_REPO env vars set.

Two paths timed:

1. **Sequential pagination** — existing `tracker.fetch_board_stories()` impl: one page at a time, per_page=100, state=all.
2. **Parallel pagination** — spawn N concurrent page fetches (hypothetical impl — not currently in tracker.rs, sketched for the spike).

For stories that currently hold structured comments (today: ~50 rework + gate-marker-active stories), fetched `list_comments` per story, measured cumulative.

## Measurements

_Populate after running._

| Path | n_stories | n_open | n_closed | wall-clock | rate-limit-remaining-before | after |
| --- | --- | --- | --- | --- | --- | --- |
| Sequential pagination | | | | | | |
| Parallel (5-way) | | | | | | |
| Comments-for-all-open | | | | | | |

Memory post-ingest (serialized JSON size as proxy):

- Full-GH-shape stories + comments: _TBD_ MB
- Datensparsamkeit-sparse stories (number + title + body + state + gate_comments only): _TBD_ MB

## Verdict signals

- **Ingest <1s sequential** → candidate A/D cheap; pick and move on
- **Ingest 1-3s** → A/D acceptable; match spec target
- **Ingest 3-10s** → A/D painful at startup; consider B or hybrid
- **Ingest >10s** → A/D infeasible; B or C load-bearing
