# phase-1b peer-advisory follow-up (#452 — peer gossip, not event replication)

Ryan 2026-04-24 15:04 UTC: single-peer scope for #451, P2P as future
consideration. 15:32 UTC: *"I do now have my second computer. we can
do this and smooth out any p2p problems over the weekend."* 18:24
UTC refined the architecture: *"each peer would be responsible for
itself. maybe we would poll github every few minutes or so for
information, and also get it on demand. everytime you update a
story, you first pull down the current stories, or something. you
would also be in contact with eachother so if you think your peer is
out of sync you can just tell them."* 18:29 UTC: *"the webrtc
connection between computers is only supposed to be message based
and nothing else."*

**Synthesized architecture**: peers are autonomous GH-cache owners.
GitHub is the shared source of truth. Peers contact each other via
WebRTC to exchange STALENESS ADVISORIES — never BoardEvents, never
replicated state. A peer receiving an advisory re-pulls from GH
through its own SourceAdapter; nothing from the peer becomes
authoritative in this peer's state.

This is NOT a CRDT event-replication design. It's a gossip-for-
freshness design that rides on top of #451's GH-centric substrate.

## 3-field spec header

**user-job:** When two (or more) bpmux peers run on different
machines, they remain in coherent view of the GH-canonical board
state with less than "next-poll-cycle" lag on the important updates.
A peer that just mutated a story tells the other peers "I touched
#N" — the other peers refresh #N on their own schedule via GH.
Owner Ryan also has a manual-refresh button (via Electron) to force
a full re-pull when he wants to.

**observation-falsifier:** I would reject if:
- BoardEvents cross the WebRTC channel (violates "message-based
  only" constraint)
- Peer-received data becomes authoritative state in the receiver's
  BoardStore (peers must re-pull from GH, not trust peer payloads)
- A peer that just mutated a story fails to advise its peers (peers
  stay silently stale until next poll tick)
- Receiving a staleness advisory doesn't trigger a targeted re-pull
  from source for the named story
- A peer without a WebRTC connection to another peer is
  catastrophically stale (poll-based fallback must still work)
- User can't manually force a refresh when they suspect staleness

**team-shape-independence:** peer count is open-ended (1 / 2 / 3 /
N). Discovery via static config for MVP. No assumption of a
coordinator peer.

## Architectural shape (advisory-only, rides on #451)

```
┌────────────────────────────────────────────────────────┐
│  MCP TOOLS LAYER (unchanged from #451)                 │
│                  │                                     │
│                  ▼                                     │
│  BOARD DOMAIN (unchanged from #451)                    │
│  - BoardStore with broadcast.subscribe()               │
│                  │           ▲                         │
│                  ▼           │                         │
│         PeerAdvisor (NEW SUBSCRIBER)                   │
│         subscribes to local BoardStore broadcast       │
│         emits advisories to connected peers            │
│         receives advisories from peers                 │
│         on receipt: calls local SourceSync.refresh_N   │
│                  ▲           │                         │
│                  │           ▼                         │
│         PeerAdvisor trait (NEW)                        │
│         advise_touched(number)                         │
│         advise_observe_stream()                        │
│         connected_peers()                              │
│                  ▲           │                         │
│                  │           ▼                         │
│         WebRtcPeerAdvisor (IMPL)                       │
│         wraps transport/webrtc.rs DataChannel          │
│         MESSAGE-ONLY framing (spec constraint)         │
└────────────────────────────────────────────────────────┘
```

`PeerAdvisor` is a BoardStore subscriber (via #451's subscribe API):
on every local-apply, it examines the event's peer_id. If
peer_id == local, it serializes an advisory (`{kind: "touched",
story_number: N}`) + sends to each connected peer over the WebRTC
channel. If peer_id != local (which doesn't happen in this design —
peers don't transmit BoardEvents), it ignores.

On receipt of an advisory from a peer, PeerAdvisor calls
`local_orchestrator.refresh_story(N)` via the local SyncOrchestrator.
The refresh re-pulls from GH, applies update-deltas to local
BoardStore, which broadcasts to local subscribers. Advisory
consumption does NOT call BoardStore.apply directly with peer-
supplied data.

**This is the architectural invariant**: peer-supplied data is never
authoritative. Every source-of-truth update flows through local
SourceSync → GH → local apply. Peers just accelerate the
discovery-of-staleness.

## Advisory message shapes

Over the WebRTC data channel, framed as JSON:

```json
{"kind": "touched", "story_number": 42, "from_peer": "work-laptop"}
{"kind": "stale_check", "story_numbers": [41, 42, 43], "from_peer": "work-laptop"}
{"kind": "stale_observed", "story_number": 42, "expected_ts": "2026-04-24T18:30:00Z", "from_peer": "work-laptop"}
{"kind": "presence", "peer_id": "work-laptop", "alive_at": "2026-04-24T18:30:12Z"}
```

- **touched** — "I just mutated this story; you might want to
  refresh." Receiver calls `refresh_story(N)`. Sent immediately on
  every local mutation (non-blocking, fire-and-forget).
- **stale_check** — "Tell me your updated-at for these stories" —
  used at reconnect to reconcile gaps. Receiver replies with its
  local updated-at timestamps + the requester can decide what to
  refresh.
- **stale_observed** — "I think you're stale on #N because my GH
  poll just saw a newer version" — sent when receiver-A's poll
  surfaces a newer story than it last advised to peer-B. Receiver-B
  refreshes #N.
- **presence** — periodic liveness heartbeat for the UI to surface
  "peer alive / peer disconnected" status.

**What's NOT sent**: BoardEvents, Story payloads, Comment bodies,
state-transitions-as-structured-data. Per Ryan's "message-based
only" constraint, peers exchange HINTS (which story + what action-
hint). Data comes from GH.

## Manual refresh affordance (#453 in-scope-adjacent)

Ryan 2026-04-24 18:29 UTC: *"you can also add a button for me to
use so I can update manually if I need to."* Filed as #453 by pm.
The button's behavior is `SyncOrchestrator.refresh_all()` — pulls
full source state + applies deltas to BoardStore. Button lives in
Electron; fires a new `mcp__bpmux-bridge__refresh_board()` MCP tool
OR a direct WS message.

#453 is scoped separately but depends on #451's refresh_all
primitive existing. #452 reuses the same primitive for staleness-
response.

## Symmetric peers diagram

See `08-internal-board-abstraction.md` → "Symmetric peers (future
#452 topology) — GH-centric + advisory" subsection for the two-peer
mirror-image shape. Both peers independently sync with GH; WebRTC
only carries advisories between them; receipt of an advisory
triggers local GH re-pull, not peer-data apply.

## Acceptance criteria

1. `PeerAdvisor` trait exists at `bpmux/src/board/peer_advisor.rs`
   with `advise_touched(number)`, `advise_observe_stream() ->
   Stream<AdvisoryMessage>`, `connected_peers() -> Vec<PeerId>`.
2. `WebRtcPeerAdvisor` implements `PeerAdvisor` over existing
   `transport/webrtc.rs` substrate. Peer discovery via static
   `~/.bpmux/peers.toml`.
3. `PeerAdvisor` registers as a BoardStore subscriber via #451's
   subscribe API. On local apply (peer_id == local), serializes
   advisory + sends to all connected peers.
4. On advisory receipt (via advise_observe_stream), PeerAdvisor
   calls `SyncOrchestrator.refresh_story(N)` — per-story targeted
   re-pull. Does NOT call BoardStore.apply directly with any
   peer-supplied payload.
5. Two-peer end-to-end test (weekend-windowed): two bpmux bridges
   on different processes, each with WebRtcPeerAdvisor pointing at
   the other. Peer A fires `create_story` + `pass_desk_check`; peer
   B receives "touched" advisories + refreshes those stories from
   GH; peer B's timeline reflects the new state within poll-ms (GH
   propagation) + WebRTC-ms.
6. Offline/reconnect test: peer A's WebRTC connection drops; peer A
   continues firing local mutations (which push to GH). When
   reconnected, peer A + peer B exchange "stale_check" advisories to
   identify divergence; each peer re-pulls affected stories from
   GH on its own.
7. Manual refresh (per #453): Electron button calls
   `refresh_board()` MCP tool, which calls
   `SyncOrchestrator.refresh_all()`. Status surfaced via updated
   `get_sync_status` output.
8. `view_peer_state` MCP tool returns connected peers + last
   advisory exchange time + presence-heartbeat drift. Sidebar in
   Electron surfaces this (future UX story).
9. Role-gate enforcement stays at the MCP-tool boundary (#438);
   peer advisories never authorize a mutation. Only advisory-
   triggered LOCAL refreshes happen; mutations only come from local
   MCP-tool-call handlers.
10. WebRTC channel framing is JSON advisory-messages ONLY. No
    BoardEvent serialization, no Story-data frames. Compliance
    verified by a unit test inspecting WebRtcPeerAdvisor's sent
    payloads.

## Advisory-flooding mitigation (in-scope MVP, per Ryan 2026-04-24 18:43 UTC)

Team-converged MVP shape for advisory-flooding prevention combines
two orthogonal techniques:

**Debounce at sender**: when a story is touched, delay sending its
advisory for a short window (e.g., 500ms-1s). If the SAME story is
touched again within the window, reset the delay. Advisory fires
only when the story is "quiet" (no touches for the window duration).
Tonight's #419 9-gate-event sequence would have collapsed to a single
advisory at sequence-end instead of 9 separate messages.

**Dedupe at receiver**: when an advisory "touched #N" arrives, check
if `refresh_story(N)` is already in-flight OR completed within the
last M seconds (e.g., 5s). If yes, drop the advisory. Prevents
pile-up when multiple peers concurrently advise the same story or
when the same peer's debounce still lets duplicates through under
edge conditions.

**Aggregate frames** (secondary): wire protocol supports
`{"kind": "touched", "story_numbers": [42, 43, 44]}` single-frame
format in addition to per-story frames, so batched bursts cost one
WebRTC message instead of N.

Combined effect: under tonight's 9-event/12-min #419 scenario, peer
would send ~1 advisory; under a 50-mutation-burst across 10 stories,
peer sends ~10 advisories (aggregated into 1-2 frames), not 50.
Simple, measurable, effective.

## Open questions (flagged for sharpening at spec-accept)

- **Missed advisories under transient disconnect**: without an
  explicit acknowledgment, a peer may miss a "touched" while
  briefly disconnected. The "stale_check" exchange on reconnect
  partially covers this; depends on keeping "last-known-advisories-
  to-each-peer" state for N minutes. MVP: periodic-full-stale_check
  on reconnect catches this.
- **GH-rate-limit sensitivity under advisory-storms**: the
  debounce + dedupe combo caps this in normal operation; if heavy-
  burst scenarios still spike GH requests, future story adds
  request-coalescing at SyncOrchestrator layer.

(Advisory-authenticity concern retired per Ryan 2026-04-24 18:43
UTC ruling: advisory carries no authoritative data; receiver pulls
from OWN GH; malicious advisory's worst outcome is a wasted API
call, not state-corruption. Non-issue for bpmux's scope.)

## Out of scope

- BoardEvent replication between peers (architectural anti-pattern
  per Ryan's messages-only constraint)
- CRDT conflict resolution (GH is the conflict-resolver)
- Cross-team peer-sync (one team per peer-set)
- End-to-end encryption of advisory traffic beyond WebRTC's
  baseline DTLS
- Dynamic peer discovery (static config for MVP)
- Webhook-push from GH (polling-only MVP)
- Event-log-compaction / WAL recovery (#451 scope; out-of-scope
  here)

## Dependency note

- **Hard dependency on #451** — PeerAdvisor registers as a
  BoardStore subscriber; #451 must land first.
- Reuses existing `transport/webrtc.rs` + `transport/peer.rs`.
- Reuses `SyncOrchestrator.refresh_story(N)` primitive from #451's
  AC #11 (refresh-before-write). Same primitive serves:
  mutation-handler refresh + advisory-driven refresh +
  manual-button-refresh (#453).
- Reuses `~/.bpmux/identity` key for stable peer-id derivation.
- Reuses existing REST signalling path for WebRTC offer/answer.
- No new external dependencies.

## Retro pointer

#452 post-reshape is fundamentally simpler than the original CRDT-
replication draft. Designer spec-round-1 (pre-Ryan-18:24) had LWW-
per-field + peer_id-lexical-tiebreak + loop-prevention + event-log-
replay. All of those are unnecessary under the GH-centric +
advisory-only model: GH is the merge point; peers don't replicate;
no loops to prevent.

Lesson for future distributed-design specs (saved in designer
memory 2026-04-24 18:27 UTC "match consistency-model to actual
usage"): match the consistency model to the ACTUAL usage, not the
theoretically-strongest-available. For bpmux's small-team + shared-
GH-source use case, "peers are colleagues nudging each other" is
the right model. CRDT would have been over-engineering.

Weekend-test design stands: both bpmux instances running on
different machines, each polling GH + exchanging advisories via
WebRTC. If the advisory channel works as spec'd, peers see each
other's mutations through GH within WebRTC-advise + GH-round-trip
ms. If the channel fails, they fall back to poll-only freshness
(slightly laggier but functional).
