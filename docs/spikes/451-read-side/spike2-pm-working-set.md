# Spike B (pm) — working-set profiling for #451

**Method**: parsed `~/.bpmux/sessions/current/messages-current.jsonl` for `2026-04-24`; extracted `#NNN` story-ID references; binned into 30-min windows.

**Caveat**: story-ID references in chat-text approximate MCP-tool-call working-set (exact MCP-invocation log would be more precise but isn't instrumented today). Over-counts in some cases (agents cite stories conversationally without invoking tools); under-counts in others (some tool calls don't surface story-IDs in chat text).

## Raw data — distinct story-IDs per 30-min window (today)

```
window              count  sample ids (first 10)
2026-04-24 00:00       30  #332, #333, #337, #341, #345, #354, #357, #358, #361, #366 (+20)
2026-04-24 00:30       40  #332, #333, #337, #341, #345, #354, #357, #358, #361, #366 (+30)
2026-04-24 01:00       12  #335, #341, #377, #378, #380, #385, #388, #393, #395, #397 (+2)
2026-04-24 01:30       36  #332, #337, #345, #347, #366, #367, #368, #370, #371, #375 (+26)
2026-04-24 02:00       18  #366, #367, #371, #385, #386, #394, #397, #420, #422, #428 (+8)
2026-04-24 02:30       23  #366, #367, #368, #371, #375, #376, #390, #410, #418, #419 (+13)
2026-04-24 03:00       23  #337, #366, #367, #371, #391, #397, #403, #410, #411, #420 (+13)
2026-04-24 03:30       10  #280, #377, #411, #412, #418, #431, #437, #438, #439, #443
2026-04-24 04:00       15  #378, #389, #394, #411, #412, #413, #414, #416, #418, #419 (+5)
2026-04-24 11:00        0
2026-04-24 11:30       24  #374, #389, #391, #410, #411, #412, #413, #414, #416, #418 (+14)
2026-04-24 12:00       20  #341, #373, #374, #389, #393, #398, #411, #412, #413, #414 (+10)
2026-04-24 12:30       31  #155, #162, #171, #184, #325, #389, #396, #411, #412, #413 (+21)
2026-04-24 13:00       14  #389, #411, #412, #432, #438, #443, #444, #445, #446, #447 (+4)
2026-04-24 14:00       10  #148, #411, #412, #418, #437, #444, #446, #447, #450, #451
2026-04-24 15:00        1  #451
```

## Total distinct story-IDs across 2026-04-24

**92** distinct story-IDs referenced (out of ~450 total stories in backlog = 20% of corpus).

## Top 15 most-referenced stories today

| Story | Refs |
|---|---|
| #431 | 75 |
| #437 | 67 |
| #443 | 58 |
| #389 | 52 |
| #429 | 47 |
| #419 | 46 |
| #411 | 45 |
| #444 | 45 |
| #445 | 43 |
| #412 | 43 |
| #418 | 42 |
| #428 | 41 |
| #430 | 41 |
| #414 | 41 |
| #420 | 37 |

Top-15 covers ~50% of all story-ID references. Long tail ≈ 77 stories with <37 refs each (many are one-off mentions).

## Findings

1. **Peak 30-min working-set: 40 distinct stories** (during tonight's heavy night-arc, 00:30 UTC)
2. **Typical 30-min window: 10-25 distinct stories**
3. **Total daily working-set: 92** (20% of the 450-story corpus)
4. **Hot-set dominance**: 15 stories account for ~50% of references; rest are long-tail
5. **Context-dense windows** (12:30 window = 31 distinct) include icebox-review references (#155, #162, #171, #184 icebox shaping stories) + active-queue mentions — reading icebox counts against the working-set

## Implications for #451 caching policy

**Eager-mirror of all 450 stories** at startup would cache 80% of data that never gets touched in a session.

**Lazy-load on story-axis** is well-fit to the observed access pattern:
- Metadata-only eager-ingest (~1s per dev's Spike 1) covers list_ready + view_board without cold-reads
- First `get_story(N)` or `view_comments(N)` triggers a cold-fetch; subsequent calls in-session hit cache
- Cold-read rate during peak window: 40 distinct stories × ~200ms per cold fetch = 8s of distributed latency across 30 min — averages to ~4 cold-reads per minute peak, which is network-budget-trivial

**Invalidation policy** can be event-driven (BoardEvent fires → invalidate cache entry for N); no TTL-based eviction needed if working-set stays under ~100. At 100-story working-set × sparse-domain (~20KB per story with body + comments loaded) = 2MB memory; negligible.

## Recommendation

Spike B supports designer's empirically-updated AC #9 + Candidate D:
- Metadata-only eager-ingest at startup
- Body + free_comments lazy-load on first read
- gate_comments eager (timeline-load-bearing for view_story_timeline which runs across many stories during audit cycles)
- BoardEvent-driven cache invalidation (no TTL)

Dev's Spike 1 finding (ingest-all infeasible at 18s) and pm's Spike B finding (working-set 10-40 per 30-min window) converge on the same shape. Pm endorses continuing with Candidate D as updated by designer's post-Spike-1 AC.
