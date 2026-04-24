# Spike 1 results: ingest-all latency benchmark

**Ran**: 2026-04-24 ~15:06 UTC against live `RyanDur/bpmux` with GH_TOKEN set.

## Numbers

| Path | n | wall-clock | size |
| --- | --- | --- | --- |
| `fetch_board_stories` (one page=100) | 100 | 820ms | 201 KB |
| `list_story_comments` (30-story sample, active only) | 68 comments | 5.69s | 68 KB |
| **Projected** comments-for-all-active (90 stories) | ~190ms/story | ~17s | ~200 KB |
| **Total eager-ingest estimate** | 100 stories + 90 comment-fetches | **~18 seconds** | ~400 KB |

**Memory footprint is NOT the constraint** — ~400KB for active stories + comments is trivial. Even a 10x projection to 4,000 comments would be <5MB.

**Ingest latency IS the constraint**. 18s for current-scale active stories exceeds the spec's 3s target by **6x**. Scaling past closed+merged stories (~400+ total) pushes this to **60-90s** for full-history ingest.

**Root cost**: `list_story_comments` per story. ~190ms/story serial. GH API issues one HTTP round-trip per story.

## Implications

Candidate A (eager-mirror of all stories + comments at startup):
- At 18s to 90s startup, **infeasible** as MVP.

Candidate A' (eager-mirror stories only, no comments):
- Stories fetch is fast enough (~4s for all ~500 via 5-page pagination, or <1s with parallel pagination). Acceptable.
- But `view_story_timeline` + `view_comments` + rework-context-enrichment all need comments. Lazy-fetch-comments-on-first-read is required anyway.

Candidate B (read-through cache with TTL) — passes:
- No startup ingest. Cold read = 190ms per comment-fetch, ~800ms for first-time `view_board`.
- Warm read = instant.
- Tradeoff: source-down makes un-read stories invisible (spec-flagged as failure mode).

Candidate C (event-sourced + local projection):
- Startup replay near-zero; source-sync is asynchronous.
- Comments can be lazy-synced behind the projection.

Candidate D (hybrid: eager story-metadata ingest + lazy body/comments) — strongest fit:
- Story-metadata ingest ~4s (one-time at startup; pagination parallel brings this to <1s).
- Body + comments lazy-loaded on first read with cache.
- Source-down resilience: story list + metadata available; body/comments degrade gracefully.

## Recommendation

Shape the BoardStore as **Candidate D with lazy body+comments**:
- Startup: full story-metadata ingest (fast, ~1s with parallel pagination) — enables `list_ready`, `view_board`, all label-based tools without cold-read.
- Lazy: body + comments fetched on first read per story; cached in BoardStore with TTL invalidation on BoardEvent.
- Future: event-sourced audit log (BoardEvent) for durability + replay — adds <10 line write-path, huge resilience win.

## Notable

- `fetch_board_stories` currently caps at per_page=100 + one page; doesn't paginate. That's a real bug in the ingest path that #451 would fix architecturally.
- Comment fetch is serial today. Parallel `futures::try_join_all` on per-story comment fetches would drop the 17s to ~3-4s.
