# Unified subscription model — dev engine/receive-path architecture (grooming go-wide)

**Status:** thin engine go-wide, dev-lane. Sibling to
`unified-subscription-model.md` (designer's conceptual-model + CLI surface).
Converges at grooming onto **#1764** (PM's scope-frame: the unified-model story;
#1753's push-wake is the landed foundation). NOT a slice-spec — PM slices the
cluster from the converged architecture+design. Grounded firsthand against the
repo (see citations).

## The key finding: "ANY event" is a naming expansion, not a receive-path one

Every event bpmux emits — chat, activity, and every `StateDelta` domain — is
already broadcast through one channel (`session.broadcast` /
`broadcast_ephemeral` → `wake_tx`), and the terminal delivery loop
(`spawn_terminal_delivery_loop`, `room/agnostic_delivery.rs`) already subscribes
to it and sees **all** of them. So the events the redesign wants to open are
already arriving at the push seam.

What's missing is **identity**: `StateDeltaPayload::to_subscription_event`
(`session/message/state_payloads.rs`) maps only ~4 domains
(`ci_state_changed`, `sync_status_changed`, `story_state_transition`,
`board_invalidated`); the rest (`reactions`, `roster`, `iteration_close`,
`iteration_close_blocked`, `board_snapshot`) return `None` — emitted but not
nameable as a subscription. So **"ANY event" = expand the event-identity map +
generalize push-delivery to all of it**, not a new receive path. Lower-risk than
it sounds: the substrate already carries everything; we're naming + routing it.

## Consolidate two registries onto one push registry

Today there are two, split by the pull/push error:

- **Bridge `SubscriptionRegistry`** (`bridge/event_subscription/`) — MCP-side,
  **pull**: `subscribe` hands back a channel that only `wait_for_event` drains.
  A dormant agent's channel just buffers. This is the design error's home.
- **Room `CommitmentStore`** (`room/commitment_store.rs`, #1753) — **push**,
  consulted by the delivery loop (`wake_delivery::wake_committed_terminals`);
  wakes a dormant agent by injecting into its terminal. One-shot
  (consume-on-fire).

The unified `subscribe` is push, so it consolidates onto the **room push
registry**: generalize `CommitmentStore` into the one subscription store the
delivery loop consults for every event. The bridge pull-registry retires with
`wait-for-event`.

## Engine slice-plan (engine view of PM's first-cut order)

1. **Foundation — unified `subscribe`/`unsubscribe` over the landed engine.**
   Generalize `CommitmentStore`: add a **standing** mode (don't consume on fire)
   alongside the existing one-shot (kept as `--once`), and an explicit
   **unsubscribe** (+ a `subscriptions` list — the engine's answer to the
   designer's lifecycle-leak risk: you can see and reap standing subs).
   `wake_committed_terminals` generalizes to deliver standing matches too.
   `subscribe --once ci_state_changed` = exactly #1753's proven path under the
   unified verb (the designer's thinnest-slice; zero new delivery-risk,
   dogfoodable as #1753 was).
2. **ANY-event generalization.** Expand the event-identity map so every emitted
   domain is subscribable; exclude self-origin events by default (no echo);
   keep the payload `--filter` (already in the engine, shared via
   `payload_matches_filter`) as the narrow-default spam-floor.
3. **Retire `wait-for-event` + the bridge pull-registry.** Remove-on-refactor
   (remove-superseded-code canon), not a lingering shim — push is the model.

## Engine answers to the designer's open questions

- **Q2 (receive-path expansion?):** No — see the key finding. Naming + routing
  expansion; the receive path already carries all events.
- **Q5 (`wait-for-event` retirement):** Remove-on-refactor, slice 3; the bridge
  pull-registry goes with it.
- **Discoverability catalog (`bpmux events`):** engine-feasible — the event
  taxonomy is enumerable from the `StateDeltaPayload` variants + message types;
  a catalog verb reads that set. In/after slice-1 is the designer+PM call.

## #1753 engine is a clean foundation

`commitment_store.rs` + `wake_delivery.rs` + the delivery-loop hook in
`agnostic_delivery.rs` stand on their own (landed on main, dogfood-proven). The
unified model **builds on** them (slice-1 generalizes `CommitmentStore`); nothing
is discarded. So closing #1753 as superseded-by-#1764 keeps the code and folds
only the standalone `wake-on-event` verb-surface into `subscribe`.
