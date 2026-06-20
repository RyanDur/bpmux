# Off-MCP event-subscription — `bpmux subscribe` design-of-record

> Designer-lane design-of-record for the off-MCP event-subscription Feature (owner-raised
> 2026-06-20: "subscribe to things so you don't rely on relayed information"). SHAPED PRIVATELY
> (designer+PM room, dev excluded) — NOT yet filed; PM surfaces the shaped story to Ryan when ready.
> CLI affordance-SHAPE + verb-grammar + falsifier = designer's lane; mechanism (the second-consumer
> dispatch) = dev's.
>
> The off-MCP CLI twin of **#1161** (the accepted MCP event-subscription tool) — REUSES its proven
> mechanism (a second consumer on the #1158 BoardEvent store-listener), does NOT re-specify its
> channel-schema. Delivers ATOP **#1753** (push-wake). Completes the agent-affordance EVENT-LOOP:
> read-parity (#1754) + push-wake (#1753) + **subscribe** (this).

## The problem (owner-raised)

The team operates **off-MCP** — the `bpmux` CLI is production, MCP is dev-only (Ryan-canon:
operate-off-MCP-via-the-CLI). So #1161's MCP `subscribe_events` tool is unusable by us. Off-MCP,
the ONLY signal is the **relay** — the turn-boundary drain that delivers EVERY queued
channel-message, and only AT a turn-boundary. It is:
- **undirected** — a firehose, not "send me only what I care about";
- **turn-boundary-only** — a dormant agent between turns gets nothing until it next turns.

`bpmux subscribe` does not exist (the phantom that cost the **#1751 CI-lag**: a dormant agent
assumed a wake that never came). The cure is a real off-MCP declarative subscription.

## The verb-grammar (mirrors #1161's shape, on the CLI; per clig.dev)

| Verb | Shape | Returns |
|---|---|---|
| `bpmux list-event-channels` | `bpmux list-event-channels [--json]` | the canonical subscribable channels + each channel's filter-dimensions (mirror #1161's `list_event_channels` schema — story.state_transitioned, story.tag_added, ci.state_changed, story.refreshed, …) |
| `bpmux subscribe` | `bpmux subscribe <channel> [--filter k=v ...]` | confirmation of WHAT was subscribed (no silent-success) |
| `bpmux unsubscribe` | `bpmux unsubscribe <channel>` | confirmation of removal |
| `bpmux list-my-subscriptions` | `bpmux list-my-subscriptions [--json]` | this agent's current subscription state (introspection) |

- **Discoverability:** `list-event-channels` is the schema source — an agent never reads source to
  learn what's subscribable. A `subscribe unknown.channel` returns a **self-describing error**
  listing the available channels (not a generic error) — clig.dev actionable-error + #1161's clause (g).
- **Per cli-design:** data→stdout / diagnostics→stderr · exit 0/1/2 · `--json` dual-mode · the
  subscribe/unsubscribe responses confirm what changed (#1161's silent-success-ambiguity clause).
- **Registry:** per-agent subscription state keyed on the **bpmux session-id** (the off-MCP identity,
  #1712) — the same key the cursor/roster use; never the mutable role-display.

## Delivery model — targeted, atop #1753 (NOT the relay firehose)

A subscribed BoardEvent reaches the agent by the SAME dispatch #1161 uses (filter+dispatch at the
#1158 store-listener — a SECOND CONSUMER, not a new event-system), but the off-MCP delivery rides
the agent's real wake-paths:
- **dormant agent → #1753's push-wake** — the subscribed event WAKES it (the #1751 cure; subscribe
  CONSUMES #1753's wake, it does not re-implement it).
- **active agent → the turn-boundary drain** — already running, the matched event drains at its
  next boundary like a channel-message.

The difference from the relay: the relay delivers EVERYTHING at a boundary; subscribe delivers ONLY
the matched event-class, and wakes a dormant agent for it. Targeted + dormant-reaching.

## Observation falsifier (the AC-discipline; PM conformance-reviews)

I would reject if:
- (a) I `bpmux subscribe` to an event-class off-MCP but receive NO notification when it fires; OR
- (b) events from OTHER classes/stories LEAK to me (scoping broken — targeted asked, firehose got); OR
- (c) a DORMANT subscribed agent is NOT woken when its subscribed event fires (the #1751 silent-miss —
  must RIDE #1753's wake, never reintroduce the phantom); OR
- (d) I can't `list-my-subscriptions` (introspection) or `list-event-channels` (discoverability); OR
- (e) `subscribe unknown.channel` returns a generic error, not a self-describing one listing channels; OR
- (f) `unsubscribe` doesn't stop the stream (a stale subscription keeps burning context); OR
- (g) subscribe/unsubscribe omit confirmation of what changed (silent-success-ambiguity).

The observation-falsifier is **behavior-pure** (a–g — what the owner/agent OBSERVES). The
**second-consumer-not-a-new-event-system** requirement is a MECHANISM-constraint (internal
architecture, not externally observable), so it lives in §Delivery model / §The verb-grammar as a
dev-grounding requirement, NOT a falsifier-clause. (PM-conformance catch 2026-06-20 — keeping the
falsifier behavior-pure; the constraint is real + load-bearing, just in its proper home.)

## Relationships

- **#1161** (accepted) — the MCP twin; this REUSES its mechanism + mirrors its channel-schema (don't
  re-specify). Sibling Feature, NOT an extension (a closed story can't take scope).
- **#1753** (Circinus, in-flight) — the push-wake this CONSUMES for dormant-delivery; sequenced AFTER
  #1753 lands. #1753 = the wake; subscribe = the interest-declaration + filter riding it.
- **#1754** (read-parity) + **#1753** (wake) + this (subscribe) = the agent-affordance EVENT-LOOP,
  off-MCP — the off-MCP completion of #1161's MCP-era capability.

## Slicing (PM's lane — noted for trace)

- **Slice A [core]:** the four verbs + filtered-delivery riding #1753.
- **Slice B [if A over-grows]:** finer event-class / per-story filters.
- Class: **Feature** (missing agent-capability, never built off-MCP — not a Bug despite the #1751-lag
  pain, per the bug-filing-gate). Future iteration, after Circinus.
