# Off-MCP event-subscription — `bpmux subscribe` design-of-record

> Designer-lane design-of-record for **#1764** (the off-MCP event-subscription Feature, owner-raised
> 2026-06-20: "subscribe to things so you don't rely on relayed information"). **REDESIGNED 2026-06-21**
> by owner direction (see §Owner-redesign). CLI affordance-SHAPE + verb-grammar + falsifier = designer's
> lane; mechanism (the unified subscription registry + push-delivery engine) = dev's (companion:
> docs/design/unified-subscription-engine.md). Converges with dev's architecture + slice-plan at grooming;
> PM slices + sequences (the detailed slice-spec is PM's).
>
> The off-MCP CLI twin of **#1161** (the accepted MCP event-subscription tool) — REUSES its proven
> mechanism. Delivers ATOP **#1753** (push-wake = the now-LANDED foundation). Completes the
> agent-affordance EVENT-LOOP: read-parity (#1754) + push-wake (#1753) + **subscribe** (this).

## Owner-redesign (2026-06-21) — two deltas + the model-correction

Ryan redesigned the model directly. **Push is THE model:** an agent subscribes to ANY event bpmux
emits, the event is PUSHED to them (waking a dormant agent, delivering to an active one), and they
unsubscribe whenever they decide. `wait-for-event` (pull-while-awake) was the **design error**. Two
deltas vs the Jun-20 shape below:

- **(a) ANY emitted event** — not the fixed #1161 channel-set. Subscription opens to the whole
  emitted-event stream (see §Event-taxonomy). Opening the not-yet-subscribable classes is engine-side (dev).
- **(b) Push is THE model** — drop the "drained-at-turn-boundary-when-active" coexistence. A subscribed
  event rides #1753's push-engine for BOTH dormant (wake) and active (inject) delivery; the undirected
  relay/turn-drain is no longer a subscription delivery-path. `wait-for-event` retires as superseded.

The conceptual model: the agent's goal is **"don't let me MISS the events I care about"** — NOT
"pull (running) vs push (dormant)." Pull-vs-push is a delivery mechanism the SYSTEM adapts; the agent
never encodes it by picking a verb (it can't predict its own dormancy). Norman: model the goal, hide
the mechanism. The one-shot-vs-standing nuance dissolves into the lifecycle: a one-shot gate is just
`subscribe → woken → unsubscribe` (no special mode — dev's resolution).

## Event-taxonomy (dev's firsthand engine-map — the 'ANY event' surface)

A subscription = **event_class + an OPTIONAL payload filter** (exact key/value match, e.g.
`story.state_transitioned story_id=1753 target_state=accepted`). The classes, categorized for the
discoverability catalog:

| Category | Classes | Subscribable | Notes |
|---|---|---|---|
| Gate/state | ci.state_changed, sync.status_changed, story.state_transitioned, board.invalidated | yes (today) | the #1751 gate-class |
| Collaboration | broadcast_received, addressed_to_me, reactions, roster | yes (opened by redesign) | chat + presence |
| Lifecycle | iteration_close, iteration_close_blocked, board_snapshot | yes (opened by redesign) | ceremony events |
| Heartbeat | sync_poll, ci_poll | **NO — excluded by design** | ephemeral liveness, not agent-actionable |

(Surface channel-naming — dotted #1161-schema vs the engine's snake event-class — reconciles at grooming;
`list-event-channels` is the agent-facing source of truth either way.)

**Dev-grounded (low-risk):** 'ANY event' is a naming+routing expansion, NOT a receive-path one — the
broadcast substrate already carries EVERY emitted event to the push seam; only the event-identity map
(`to_subscription_event`) is partial (~4 of the domains). Opening the rest = extending that map,
engine-side — lower-risk than it reads.

## The verb-grammar (mirrors #1161's shape, on the CLI; per clig.dev)

| Verb | Shape | Returns |
|---|---|---|
| `bpmux list-event-channels` | `[--json] [--subscribable]` | the canonical subscribable channels + each channel's filter-dimensions (the discoverability catalog; `--subscribable` hides heartbeats) |
| `bpmux subscribe` | `<channel> [--filter k=v ...] [--once]` | confirmation of WHAT was subscribed (no silent-success) |
| `bpmux unsubscribe` | `<channel>` \| `--all` | confirmation of removal |
| `bpmux list-my-subscriptions` | `[--json]` | this agent's current standing subscriptions (introspection + the leak-guard) |

- **`--once` = the gate-sugar** — auto-unsubscribe on first match (the one-shot = subscribe→woken→
  unsubscribe lifecycle in one line). NOT a second verb; sugar over the standing lifecycle. This is how
  #1753's gate-case re-surfaces under the unified verb.
- **`--filter k=v`, AND-combined, exact-match** (the engine's seed). Floor only — no ranges/sets/operators
  until a real need surfaces (YAGNI). Omit to subscribe to the whole class.
- **Discoverability:** `list-event-channels` is the schema source — an agent never reads source to learn
  what's subscribable, nor which keys `--filter` accepts. `subscribe unknown.channel` returns a
  **self-describing error** (exit 2) listing the channels (clig.dev actionable-error; clause e).
- **`list-my-subscriptions` is the leak-guard** — standing subs are VISIBLE, so a forgotten subscription
  is seen (by the agent and a reviewer), not a silent spam-source. Pairs with the riskiest-assumption.
- **Per cli-design:** data→stdout / diagnostics→stderr · exit 0/1/2 · `--json` dual-mode · subscribe/
  unsubscribe confirm what changed (silent-success-ambiguity clause).
- **Registry:** per-agent state keyed on the **bpmux session-id** (#1712 off-MCP identity), never the
  mutable role-display.

## Delivery model — push is THE model, atop #1753's LANDED engine

A subscribed event reaches the agent via ONE unified subscription registry consulted by the broadcast
delivery loop (the #1753 wake-engine generalized — inject into the terminal, wake if dormant):
- **dormant agent → #1753's push-wake** — the subscribed event WAKES it (the #1751 cure).
- **active agent → push-inject** — the matched event injects into its terminal directly (NOT the
  turn-drain coexistence, which the redesign drops).

The difference from the old relay: the relay delivered EVERYTHING at a boundary; subscribe delivers ONLY
the matched event and reaches a dormant agent. Targeted + dormant-reaching + push-native.

## Designer-craft decisions (the shaping Ryan delegated)

1. **Heartbeats are NOT subscribable** (sync_poll / ci_poll) — internal liveness, fire constantly, no
   agent-actionable signal; the catalog marks them excluded, the engine rejects a subscribe to them.
2. **Exclude self-origin by default** (echo-prevention) — an agent isn't woken by its OWN emitted event.
   → ENGINE-QUESTION for dev: is event-origin on the payload (filterable) or does it need an origin-tag?
3. **Narrow default; filter = the volume-control** — no-filter wakes on every event of the class (fine
   for sparse ci, spammy for dense board.invalidated); the catalog surfaces typical volume so the agent
   filters dense classes. Never a "subscribe to everything" convenience.
4. **Exact-match filter is the validated floor, not the ceiling** — ships the proven engine-seed; richer
   operators are a deliberate later go-wide, not slice-1.

## Riskiest assumption + thinnest slice (lean)

**Leap-of-faith:** that the *standing, agent-controlled* lifecycle is reliably managed — agents subscribe,
get pushed, and **actually unsubscribe** rather than leaking standing subs that spam them forever. If leak
is common, standing is worse than #1753's auto-consumed one-shot.

**Thinnest slice:** `subscribe --once` on ci.state_changed — #1753's *proven* wake-engine re-surfaced under
the unified verb, dogfooded exactly as #1753 was (zero new delivery-risk). Ships the unified SURFACE + the
lifecycle before standing-subscriptions, the full ANY-event opening, and the catalog. `list-my-subscriptions`
(the leak-guard) is the cheapest insurance and pairs naturally with slice-1. (Aligns with PM's first-cut
order: foundation → ANY-event generalization → wait-for-event deprecation.) Dev confirms slice-1
generalizes the #1753 CommitmentStore to standing + one-shot (`--once`) + explicit unsubscribe + the
`list-my-subscriptions` leak-guard — so the engine SUPPORTS the riskiest-assumption's mitigation.

## Observation falsifier (the AC-discipline; PM conformance-reviews)

I would reject if:
- (a) I `bpmux subscribe` to an event-class off-MCP but receive NO notification when it fires; OR
- (b) events from OTHER classes/stories LEAK to me (scoping broken — targeted asked, firehose got); OR
- (c) a DORMANT subscribed agent is NOT woken when its subscribed event fires (the #1751 silent-miss —
  must RIDE #1753's wake, never reintroduce the phantom); OR
- (d) I can't `list-my-subscriptions` (introspection) or `list-event-channels` (discoverability); OR
- (e) `subscribe unknown.channel` returns a generic error, not a self-describing one listing channels; OR
- (f) `unsubscribe` doesn't stop the stream (a stale subscription keeps burning context); OR
- (g) subscribe/unsubscribe omit confirmation of what changed (silent-success-ambiguity); OR
- (h) **[redesign]** a class outside the old fixed #1161-set (e.g. roster, iteration_close) can't be
  subscribed though it's emitted + catalogued; OR
- (i) **[redesign]** a HEARTBEAT (sync_poll / ci_poll) IS subscribable (should be excluded), OR an agent
  is woken by its OWN emitted event (self-echo); OR
- (j) **[redesign]** `subscribe --once` does NOT auto-unsubscribe after the first matched delivery.

The falsifier is **behavior-pure** (what the agent OBSERVES). The unified-registry / second-consumer
requirement is a MECHANISM-constraint (dev's §engine-companion), not a falsifier-clause.

## Relationships

- **#1161** (accepted) — the MCP twin; REUSES its mechanism + mirrors its channel-schema (don't re-specify).
- **#1753** (LANDED on main) — its push-wake ENGINE is the now-landed foundation this builds on (consumed,
  not discarded). Its separate `wake-on-event` VERB is **superseded by this unified subscribe** → #1753
  closes-as-superseded-by-#1764 (confirm at grooming once the slice-plan confirms the engine stands cleanly
  as the foundation). Designer holds `pass_desk_check` on #1753 meanwhile (don't bank the superseded verb).
- **#1754** (read-parity) + **#1753** (wake) + this (subscribe) = the off-MCP agent-affordance EVENT-LOOP.
- **`wait-for-event`** — retires as the design-error, **remove-on-refactor** (dev-confirmed: push is the
  model); the bridge-side PULL SubscriptionRegistry retires with it. Unified subscribe consolidates the two
  registries (bridge pull + room push #1753) onto the ROOM push registry.

## Slicing (PM's lane — noted for trace)

- **Slice A [core / foundation]:** unified `subscribe`/`unsubscribe` + `--once` + filtered-delivery riding
  #1753's landed engine + `list-my-subscriptions` (the lifecycle = the one-shot). PM's first-cut foundation.
- **Slice B:** ANY-event generalization (open the not-yet-subscribable classes; engine-side) + the
  `list-event-channels` discoverability catalog.
- **Slice C:** `wait-for-event` deprecation/migration. (+ finer per-class/per-story filters if needed.)
- Class: **Feature** (missing agent-capability). Future iteration, after Circinus; icebox until its IPM.
