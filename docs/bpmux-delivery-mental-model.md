# bpmux delivery mental-model — events + deploy

Why this doc: at the Aquila retro (2026-06-07) all three agents independently
admitted they carried a *private, incomplete* mental-model of how bpmux
delivers — events and deploy. That gap drove real cost: the relay-SPOF
overnight stall, the GitHub rate-limit from poll-thrash, and the #1518
rebuild-cycles spent guessing "is it live?". This is the shared, substrate-
grounded model so we stop re-deriving it. Mechanics grounded firsthand by dev
in `event_subscription.rs` + `connection.rs` and against the running artifact.

This doc is the "is it live / did it arrive?" reference for the master canon
*verify the lived reality at its effective locus, never a proxy* — the DEPLOY
and EVENT-DELIVERY axes specifically.

## Event delivery — three tiers

The bridge does NOT push everything to your LLM stream. Three tiers:

1. **ALWAYS-PUSHED (passive, reliable, can't-mute)** — messages addressed to
   you, owner (Ryan) messages, broadcast-chat. These reach your stream
   passively. *This is why coordination worked all session — nothing was lost
   on this tier.*

2. **CHAT-RELEVANCE classes (passive, tunable)** — `chat` / `insight` /
   `nudge`. These tune your live chat push-stream. Declare a relevance-set via
   `subscribe` and your chat push carries only those classes; declare nothing
   and a curated default applies. Addressed-to-you + owner messages are always
   pushed regardless.

3. **STRUCTURED classes (PULL-by-design, NO passive push)** —
   `ci_state_changed`, `board_invalidated`, `story_state_transition`,
   `sync_status_changed`. The mechanism:
   - `subscribe(class)` → REGISTERS a per-subscriber unbounded channel (tx/rx).
   - `dispatch(class, payload)` → when an event fires, SENDS to matching
     subscribers' channels — events QUEUE there, they are NOT lost.
   - `wait_for_event(subscription_id)` → BLOCKS on `rx.recv()` (with timeout)
     to drain the queue. **This is the only way to receive structured events.**
   - There is **NO code path that passively injects structured events into the
     LLM stream.** So `subscribe()` WITHOUT a `wait_for_event` = events pile up
     unconsumed = SILENT. This is by design (the `subscribe` tool-desc says
     "structured classes pair with `wait_for_event`").

   The empty `bpmux:` state-deltas you see flooding the stream are a SEPARATE
   content-free broadcast — NOT the structured subscription firing.

### Consequence + the interim discipline

You cannot passively gate on CI/board/state while doing other work. The only
options today are BLOCK (`wait_for_event`), POLL, or RELAY (a peer forwards) —
the relay was the SPOF that stalled us overnight.

**Interim discipline — PULL-AT-GATE:** at the decision-moment, pull the state
you need (`get_ci_state` / `view_board` / `view_verdicts` / `get_story`). Pull
ONCE at the gate — do NOT busy-poll in a loop while waiting (the GitHub
rate-limit was poll-thrash). Never read "no push" as "no event"
(green-attests-does-not-falsify). The idle-wait is exactly what the future
passive-push (#1631) replaces; until it ships, pull-at-gate is the floor.

## Deploy — per-artifact, independent (probe the running artifact, never infer)

There is no single "deploy". Each artifact propagates differently; "is it
live?" is answered by probing the running artifact (capture / computed), never
inferred from "committed" or "the dev-server serves it":

- **Rust binary** (the bridge + MCP tools) ← `cargo build` + **restart**. A
  code change to a bridge tool is NOT live until rebuilt and the bridge
  restarts. (This is why post-rebuild live-fire confirmed the reject-record
  tools that pre-rebuild verdicts had to defer.)
- **Built renderer bundle** (`out/renderer`, what a packaged Electron loads) ←
  `npm run build`. OR, if the owner is running `npm run dev`, vite-dev HMR
  applies a renderer/CSS change live — BUT an already-open modal/view keeps the
  stale render until re-opened.
- **vite dep-cache** (`node_modules/.vite`) — irrelevant to a *built* Electron;
  clearing it does not change the built bundle.
- **`cargo clean`** wipes `target/debug/bpmux`, which vitest's globalSetup
  requires — rebuild with `cargo build -p bpmux` before running vitest.

Rule of thumb: a renderer-CSS change behaves differently under packaged-build
vs `npm run dev` HMR vs an already-open view. When in doubt, **probe the
running artifact** (capture_electron_window / live computed-style) — do not
infer liveness from the commit or the served bundle.

## See also

- `docs/memory-architecture.md` — the durable knowledge-chain this feeds.
- The master canon (team lesson, 2026-06-07): *verify the lived reality at its
  effective locus, never a proxy — mechanize per-axis.* This doc is the DEPLOY
  + EVENT-DELIVERY axes' substrate reference.
- #1631 (Epic) — passive structured-push, the durable fix for the EVENT tier.
