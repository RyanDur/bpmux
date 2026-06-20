# Attention-signal — go-wide design exploration (#1753 agent-affordance theme)

> Owner-directed "your attention is needed HERE." Mockup-first, owner-reviewed.
> Designer go-wide; Ryan reviews at his discretion (next-iteration to build).

## The need (grounded)

An agent needs to signal the **owner** that *your attention / decision is needed here* —
a specific story, agent, or decision-point — so Ryan sees it **without scanning every room**.

Empirical: the #1751 CI-lag. The agent was silently-blind; Ryan had to ask "what's going on."
Had there been an attention-signal, the agent could have flagged "owner-attention-needed"
when it stalled — Ryan sees it, instead of discovering the stall himself.

This is the **inverse of presence**: presence = agent-state the owner *observes*; the
attention-signal = an agent *requesting* the owner's attention. Sibling to the needs-help
dot (S3) — but that is agent↔agent; this is **agent→owner**.

## Go-wide — three affordances (what marks it / where it surfaces / how it clears)

### Option A — Room-attention dot (the existing room-nav)
```
ROOM LIST
  ● all
  ● designer + pm   ◆   <- needs-your-attention
  ● dev             ◆
  ○ board
```
- WHAT: a room gets a ◆ marker when an agent in it flags owner-attention.
- WHERE: the room-list (Ryan scans it; the ◆ rooms need him).
- CLEARS: Ryan visits the room / acknowledges; or auto on the agent's next action.
- + minimal new surface, fits the room model.  – Ryan still scans; room-level, not item-specific.

### Option B — Owner-attention tray (aggregated, "Needs You")
```
  NEEDS YOU (3)
  +------------------------------------------------+
  | * dev      — #1751 CI green, awaiting land-OK   |
  | * designer — dot shows red, is this right?      |
  | * pm       — #1753 class: confirm Feature?      |
  +------------------------------------------------+
  one place; each item = who + what + the decision; click -> go there
```
- WHAT: a dedicated owner surface that AGGREGATES every pending owner-attention item.
- WHERE: one top-level tray Ryan opens — one place to see all that needs him.
- CLEARS: Ryan acts on the item (acknowledge / decide) -> it leaves the tray.
- + ONE place, no scanning, item-specific, scales with team-size.  – a new surface + plumbing.

### Option C — Board attention-flag (story-level)
```
  BOARD
  Ready     Desk-check     Acceptance
  #1684     #1750  ◆       #1747
  #1705                    #1748
```
- WHAT: a story-card gets a ◆ flag when an agent flags it.
- WHERE: the board (Ryan's primary surface; flagged cards stand out).
- CLEARS: Ryan acts on the story / the agent un-flags.
- + fits the board, story-specific.  – only story-tied attention (not "is my dot red?"); board-gazing required.

## Recommendation (converge): hybrid — in-context signals FEED one tray

The core need is "Ryan sees what needs him **without scanning**." Option B (the tray) serves
that best — aggregated + specific + scales. But A and C are the **in-context** indicators that
should FEED B: a room-dot (A) and a board-flag (C) mark *where* it is, and they roll up into the
**Needs-You tray** (B) as the one-place-to-look. So: in-context signal + a single aggregate.

- **Desirability**: high — directly removes "Ryan has to discover stalls himself."
- **Feasibility** (dev): the agent->attention-item plumbing rides the same roster/presence push
  already built (#1739); the tray is a new render surface, the in-context dots reuse the dot model.
- **Viability**: it is the substrate-honest companion to #1753's event-wake — agents wake on events
  (#1753); the owner is woken on agents' needs (this). Same collaboration-model, both directions.

## Riskiest assumption — for Ryan's mockup-review (the thinnest slice to validate)

1. **Shape**: do you want the ONE-place aggregate (the Needs-You tray), the IN-CONTEXT signals
   (room-dot / board-flag) and you're fine scanning, or BOTH (the hybrid)?
2. **Trigger**: what RAISES a signal — an agent EXPLICITLY flagging "I need Ryan," or AUTO-derived
   from a blocked / owner-decision / awaiting-land state?
3. **Clear**: owner-acknowledged, or auto on the agent's next action?

These are owner-need questions — they pick the build-shape. No rush (next-iteration to build);
this go-wide is the readiness, Ryan engages at the review whenever he turns to it.
