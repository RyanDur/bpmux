# Designer Spike 3 — timeline-reconstruction walk for rejected-and-reworked story

Scenario: #419 tonight. Actual sequence of gate-events from our session:

| ts (UTC)     | gate              | actor    |
|--------------|-------------------|----------|
| 11:40:02     | filed             | (pre-session) |
| 11:40:50     | pickup            | dev      |
| 11:43:00     | delivered         | dev      |
| 11:43:30     | fail_desk_check   | designer (REJECT — 8 inline comments) |
| 11:44:23     | delivered         | dev (rework-1) |
| 11:45:15     | fail_desk_check   | designer (REJECT — #418 scope bleed) |
| 11:49:42     | delivered         | dev (rework-2) |
| 11:51:34     | pass_desk_check   | designer |
| 11:51:45     | accepted          | pm       |

9 gate-events across 12 minutes. Rapid mid-session activity with two
designer-REJECTs and one dev-rework-per-reject.

A teammate running `view_story_timeline(419)` at varying times during
this sequence experiences different completeness depending on the
read-side shape.

## Candidate A — eager-mirror + 60s poll

**Observed behavior** at t=11:44:15 (15s after fail_desk_check comment):
- If last ingest-all fired at 11:44:00, the comment is NOT yet in the
  mirror. Timeline shows filed+pickup+delivered only. Missing: the
  just-fired REJECT.
- If last ingest-all fired at 11:43:30 (coincident with comment-post),
  RACE — timeline may or may not include the comment depending on
  GitHub's read-after-write timing.
- Mitigator: the MUTATOR's local state updates synchronously via
  BoardEvent enqueue. So if designer fires `view_story_timeline`
  immediately after their own `fail_desk_check`, they see it. But
  pm looking at the same story at the same moment sees the pre-REJECT
  state.

**Cross-bridge coherence gap**: up to 60s. If pm reacts to their
stale view by commenting mid-REJECT-rework window, they may present
state that's already changed.

## Candidate B — read-through cache with TTL

**Observed behavior** at t=11:44:15:
- Pm fires `view_story_timeline(419)` (pm hasn't accessed this story
  this session). Cache miss → source fetch → latest state returned.
  Timeline includes REJECT.
- Designer fires same call 2s later. Cache hit → returns the cached
  state from pm's fetch. Still fresh.
- At t=11:46:00 (2 min later, after rework-1 delivered), timeline may
  or may not include rework-1 depending on TTL.
- TTL-default-5min: rework-1 NOT in view (cache still from 11:44:15);
  stale.
- TTL-default-30s: fresh, but higher cold-miss rate.

**Cross-bridge coherence gap**: depends on TTL; tunable but not free.
TTL short → GH API pressure. TTL long → more stale reads.

**Source-down behavior**: uncached reads fail. Team loses visibility
on stories they haven't fetched this session, regardless of whether
the story is currently being actively discussed.

## Candidate C — event-sourced projection

**Observed behavior** at t=11:44:15:
- Designer fired fail_desk_check at 11:43:30. This wrote a BoardEvent
  locally (instantly) + enqueued source-push.
- Designer's local timeline at 11:44:15 shows the REJECT (local event
  applied).
- Pm's local bridge does not yet have the event — pm's timeline at
  same moment depends on whether sync-in polled between 11:43:30 and
  now.
- Cross-bridge: same ~60s lag as Candidate A because peer-bridges
  still learn via source-ingest or observe_changes stream.

**Source-down behavior**: local mutations succeed + queue for push;
local timeline always shows OUR truth. Cross-bridge drift grows but
doesn't break individual-bridge usability.

**Additional property**: the BoardEvent log is an authoritative audit
trail. `view_story_timeline` becomes a projection-of-events-function,
not a parse-comments-from-source function. The parse-comments-with-
markers brittleness (today's `classify_marker` in view_story_timeline.rs)
goes away — markers become structured events at write-time.

## Candidate D — hybrid A+C

**Observed behavior** at t=11:44:15:
- Local mutations flow through BoardEvent log (C's property).
- Startup + 60s poll keeps the mirror warm for cross-bridge visibility
  (A's property).
- Designer sees their REJECT instantly; pm's view lags up to 60s;
  BUT pm still sees the pre-REJECT state coherently (doesn't get a
  cache-miss error during source-down).

**This is what tonight's session actually ran on** — except the
"BoardEvent log" is scattered across comment-markers in GH, and the
"60s poll" is ad-hoc fetch calls via the tracker trait. D is
essentially the architectural articulation of what we already built
organically.

## Designer-lens verdict

**D is most coherent for team-coordination** because:

1. **Mutator-instant feedback** (from C): designer's REJECT is visible
   to designer immediately. No "did my fail_desk_check go through?"
   polling.
2. **Cross-bridge visibility with bounded lag** (from A): peer bridges
   see state within 60s, acceptable for bpmux's cadence.
3. **Source-down-survives** (from A): GH outage doesn't hide stories
   the team has been working on; board serves last-known-good.
4. **Audit-trail for free** (from C): BoardEvent log replaces the
   current comment-parsing pattern; `classify_marker` brittleness
   goes away; gates are typed events, not prose-matched strings.

**Candidate B (read-through cache) loses this walk specifically
because tonight's scenario had 9 gate-events in 12 minutes across a
story being held across 3 roles. Any TTL choice is wrong:**
- Short TTL (30s) → GH rate-limit pressure under rapid activity
- Long TTL (5min) → stale across whole REJECT-rework cycles
- Even "invalidate on BoardEvent push" (cache-aside pattern) needs
  BoardEvent infrastructure — so you're partway to C anyway.

## Field-minimization cross-check (pm's parallel task)

Grep of `bridge/tools/*.rs` for GH JSON keys actually read:

- `title`: list_ready, view_backlog → **KEEP in domain**
- `body`: get_story, rework_context → **KEEP (lazy-load candidate)**
- `state`: list_ready filter, view_backlog column-derivation →
  **KEEP (typed StoryState enum; no raw string)**
- `labels`: list_ready return, view_backlog return, dev_in_flight
  state-check, rework_context is-rework-check
  → **MORE THAN JUST STATE-DERIVATION.** list_ready + view_backlog
  currently return raw labels to MCP callers. Callers today see
  substrate-encoding. Drop the leak: migrate list_ready + view_backlog
  to return StoryState instead. Internal state-checks use enum.
- `created_at` on Story: view_story_timeline `filed` event →
  **KEEP (as filed_at)**
- `user.login` on Story: view_story_timeline `filed` actor →
  **KEEP (as filed_by short-form)**
- `created_at` on Comment: view_story_timeline + rework_context
  → **KEEP (GateComment.ts / FreeComment.ts)**
- `body` on Comment: view_story_timeline + rework_context +
  view_comments → **KEEP**
- `user` on Comment: view_story_timeline actor / view_comments
  author → **KEEP (short-form)**

**Label-leak fix is in-scope for #451**: list_ready + view_backlog
migrations to StoryState return-shape. Adds ~10 lines to the story's
AC list; closes the substrate-leak into MCP return payloads.

## Spike-output summary

- Recommend **Candidate D** (hybrid eager-mirror + event-sourced).
- Recommend adding the **label-leak-fix** to #451's AC list: MCP
  handlers return StoryState, not raw labels, post-reshape.
- Spike-1 (dev) measures whether eager-mirror startup-latency is
  acceptable. If yes, D is viable. If no, fall back to C (pure event-
  sourced, lazy source reads).
- Spike-3 (dev) measures BoardEvent log volume — informs WAL size +
  replay cost for C/D.
