# Attention-signal design-language — the one vocab across the presence/help/attention cluster

> Designer-lane connective-tissue (Circinus, agent-affordance theme). The presence dots, the
> needs-help/stuck backstop, #1753's dormant-park, and the owner-attention signal were each
> designed separately; this is the ONE coherent vocab they all read off — so a new signal-state
> slots onto a grid, never bolts on. Pairs with dev's ONE presence/signal substrate (/api/presence
> roster + the #1739 pushed-state path) the same way the roadmap read-view pairs with its
> backend-derive: this is the read-parity vocab, dev owns the derive.
>
> RECONCILES (does NOT re-specify — each state's detail lives in its source doc):
> `roster-membership-model.md` (membership + the dot) · the #1739 slices (#1747/#1748/#1750)
> + #1721 (the pushed-dot) · `input-ownership-1656.md` §Signal-states (the needs-help/stuck
> backstop) · `attention-signal-gowide-mockup.md` (owner-attention).

## Why one vocab

Four signal-surfaces grew up separately and started to overlap (the #1721 history is a whole
roster of name-keyed signals colliding). Without one vocab, each new state — dormant-park for
#1753, owner-attention — risks a bolted-on color/glyph that contradicts a sibling. The cure is a
single grid with two discriminating axes; every state names its cell, so the system is
enumerable and a new signal is placed, not invented.

## The two axes

1. **DIRECTION — who the signal is FOR:**
   - **OBSERVED (passive):** the owner/peers OBSERVE the agent's state. No request is made; the
     agent just IS busy/idle/parked. Surface = the roster dot. (This is "presence".)
   - **DIRECTED (active):** the agent REQUESTS attention — to a PEER (needs-help) or to the
     OWNER (owner-attention). Surface = a dot the request-target reads, and for the owner an
     aggregate ("Needs-You"). (This is the "inverse of presence" the go-wide doc named.)
2. **SUBJECT — what is signalled:** liveness/availability (am I here, working, free, parked,
   gone) vs a need (I need help / I need the owner).

`needs-help` is the **hinge**: it is BOTH an observed dot-state AND a directed peer-request —
which is exactly why it is the load-bearing-first state (the seam the #1656 backstop and the
dot-cluster both build on).

## The unified state-grid

### PRESENCE — OBSERVED (the roster dot; over the membership layer)

Membership (who appears) = connectedness, keyed by routing-id — `roster-membership-model.md` layer 1.
The dot is layer 2 over each member:

| State | Meaning | Visual (placeholder; tokens at Darcula) | Clears / transitions when |
|---|---|---|---|
| **offline** | disconnected — the roster-leave path | dim | the agent reconnects (re-appears) |
| **unknown / present** | connected, terminal-state not yet resolved (honest "here, not yet known") | hollow | the terminal resolves → busy/idle |
| **busy** | actively working (pushed busy / terminal active) | solid, working-color | quiet / Stop → idle |
| **idle** | connected, quiet, AVAILABLE ("free-to-talk") | solid, confident green | a prompt → busy; an event-commit → dormant-park |
| **dormant-park** ⟵ NEW (#1753) | committed to a fire-on-event gate, PARKED waiting for an external wake — healthy-but-waiting BY DESIGN | distinct from idle (NOT available-now) and offline (NOT disconnected) — a parked treatment, not green, not dim | the awaited event fires → wakes → busy |
| **needs-help / stuck** (hinge) | a permission/help is pending, OR the agent is hung / not-input-ready (the #1656 agent-liveness backstop) | attention-color (the dot-cluster's needs-help) | the help is answered / the agent resumes consuming input |

`dormant-park` is the one genuinely-new PRESENCE state, and the grid shows why it needs its own
cell: it is neither idle (idle = available right now) nor offline (offline = gone) — it is
connected-and-intentionally-waiting. Reading a dormant agent as idle would say "free to talk"
when it is parked on an event; reading it as offline would fabricate absence. Distinct cell.

### ATTENTION-REQUEST — DIRECTED

| State | Direction | Meaning | Surface | Clears when |
|---|---|---|---|---|
| **needs-help** (hinge) | agent → PEER | I need a teammate's help (the dot's needs-help, agent↔agent) | the roster dot (above) | a peer/owner resolves it |
| **owner-attention** ⟵ NEW (Ryan-gated) | agent → OWNER | "your attention / decision is needed here" (a story / agent / decision-point) | in-context (room-dot / board-flag) FEEDING one aggregate "Needs-You" tray — `attention-signal-gowide-mockup.md` hybrid | the owner acts / acknowledges (or auto on the agent's next action) |

owner-attention is the escalation needs-help cannot serve: needs-help reaches a *peer*; when the
blocker is the *owner's* (a decision, a land-OK, the #1751 CI-lag class), the agent directs the
owner specifically — without the owner scanning every room.

## The governing bar — trustworthy-to-act (across EVERY state)

Ryan's north-star (Circinus #1728): a signal's entire value is trustworthiness — an untrustworthy
one is NEGATIVE (it trains the owner to ignore the affordance). Every state above must pass:
- **trust-enough-to-ACT without double-checking** — the owner answers a needs-help, leaves a busy
  agent alone, goes to an owner-attention item, all without re-verifying;
- **CLEARS when the condition resolves** — needs-help clears on grant, dormant-park clears on wake,
  owner-attention clears on owner-act. A signal that lingers past its cause is false;
- **NEVER false-positive** — an idle agent never reads busy; a parked agent never reads stuck; an
  agent with nothing pending never raises owner-attention.

This is the desk-check bar for the whole cluster (the trustworthy-dot acceptance-bar applied to
every signal, not just the dot).

## Build-staging — consumer-aligned (the states ship where their consumers land)

The vocab is authored WHOLE (this doc); the states BUILD where their consumers land — so a later
state slots onto a defined cell, no retrofit:
- **needs-help / stuck — load-bearing-FIRST** (Circinus): dev builds the needs-help backstop first
  as the shared seam; the #1656 input-ownership backstop + the dot-cluster (#1747/#1748/#1750/#1721)
  both conform to this state. Defined in `input-ownership-1656.md` §Signal-states.
- **busy / idle / offline / unknown** — the existing pushed-dot states (#1739 slices + #1721), in
  the Circinus dot-cluster.
- **dormant-park** — rides **#1753** (Circinus wake-spike, rank 13): the wake transitions
  dormant-park → busy. The cell is defined here so #1753's build renders it consistently.
- **owner-attention** — **Ryan-gated**; the go-wide (`attention-signal-gowide-mockup.md`) awaits
  the owner's mockup-review (shape/trigger/clear). Likely a later-iteration build; the cell is
  reserved here so it lands coherent with the rest.

## Pairing with dev's substrate (read-parity shape)

dev owns ONE presence/signal substrate — the /api/presence roster + the #1739 pushed-state path
(hooks → signal → roster Presence). This vocab is the **read-parity** over that single derive: the
states here are what the substrate's pushed-state resolves to, rendered consistently across the
dot, the tray, and the board-flag. Same backend-derive ↔ read-parity shape as the roadmap view
over its derive — one substrate, one vocab, many surfaces. A surface that invents a state outside
this grid (or derives one its own way) breaks parity, exactly as a roadmap surface that
over-renders breaks read-parity.

## Not in this vocab (boundary)

The **advisory-danger signal** (`bpmux-advisory-signal-substrate/research.md`, #1346 — signaling
data-integrity / process dangers without prescribing team-process) is a SEPARATE concern, not an
attention-signal state. It signals about the BOARD/data, not about an AGENT's state/need. Kept
distinct so the two "signal" vocabularies don't bleed.
