# Shared view architecture — one derivation, layered, for humans *and* agents

> Owner-steered (Ryan, 2026-06-28), captured by dev. The durable reference
> for how bpmux's views (roadmap first) are derived and served. Reverses
> the earlier "frontend derives the view" spine — read this before touching
> roadmap/view code. Wants PM/designer eyes when the bridge is back up.

## The point

bpmux exists to help **humans and agents work together**. That only works
if **both sides see the same picture** — the same roadmap, the same
iteration state, the same stories. So a view is not a per-client artifact;
it is **one derivation, served identically to every client**: the Electron
renderer (humans), the `bpmux` CLI (agents/humans), and the MCP tools
(agents). One source of truth, shared context.

## What's wrong today (the divergence this fixes)

The roadmap is derived **three different ways**:

- **Renderer**: `selectRoadmap.ts` derives rich cards (why / tone /
  readinessState / nested iterations) from the outcomes delta.
- **Backend domain**: `product_management/roadmap/{view,drill}.rs` derives a
  *different*, thinner CLI model (`OutcomeState`, bands) — and it lives in
  the **domain layer**, which is the wrong home for presentation.
- **Agents (MCP `view_roadmap`)**: read a **static `ROADMAP.md` file** off
  disk — not a live view at all.

Three derivations, three answers. Humans and agents are *not* looking at the
same thing. And a naive fix (push the whole nested tree to everyone) would
**saturate an agent's context and burn tokens** in one shot.

## The architecture

### 1. One derivation, on the backend (single source of truth)

The backend is the only place a view is derived. Every client **consumes**
the served view; no client re-derives it. The Electron store stops deriving;
the CLI/MCP stop their own derivation / static-file reads.

> This reverses the `f4fb1bc5` "frontend derives via `selectRoadmap`, wire
> serves domain" direction. Reason: a TS renderer and Rust CLI/MCP cannot
> share one implementation, so a *single* source of truth means deriving
> once (Rust, backend) and serving the result to all.

The derivation lives in a **backend view layer**, not in
`product_management/` (the domain stays pure; presentation is not domain).

### 2. Layered, lazy, addressed by parent id

A view is **not** one nested tree. It is a stack of layers, each pulled on
demand by the parent's id:

| Layer | Request | Returns |
| --- | --- | --- |
| Roadmap | `roadmap` | all **outcomes**, flat, in **priority order**; each carries its **horizon** (now / next / later) and its **rolled-up readiness** |
| Iterations | `outcome/{id}` | that outcome's **iterations** |
| Stories | `iteration/{id}` | that iteration's **stories** |

You see the roadmap shape (what's now/next/later, which outcomes are ready
or have gaps) from the **top layer alone**. You pull the iteration layer
only to see *why* an outcome has gaps; you pull the story layer only when you
drill into an iteration. An agent never receives the whole tree in one shot —
**it pulls exactly the layer it needs**, conserving context and tokens. A
human drills the same way.

Rolled-up readiness is a **derived domain fact** computed by the backend and
carried *in the outcome layer*, even though the iteration detail is a
separate fetch. The card says "ready / N gaps"; the next layer says why.

### 3. Wire discipline — hydrate then delta, never a snapshot

Per layer: **hydrate once on demand**, then push **only the targeted delta**
when something changes while you're looking at it. Never the entire app
snapshot, never a god-shot, never a full re-push of a layer on every change.
This is the existing roster-style discipline, applied per layer.

### 4. The client store collapses to thin

With derivation on the backend and the wire serving ready-made layers, the
client store has no reducers and no derivation left. It becomes a **thin
hydrate + merge cache** of the served layers: `useSelector` reads a slice,
`useDispatch` sends actions to the WS. The same shape serves humans and
agents equally. (Whether this is still "redux" or a hand-rolled
`useSyncExternalStore` cache is a detail we settle *after* the first layer
proves it — do not rip redux out speculatively.)

## Consequences

- **Retire** `selectRoadmap`'s derivation in the renderer → a thin read of
  the served roadmap layer.
- **Retire** `product_management/roadmap/{view,drill}.rs` from the domain;
  the derivation moves to the backend view layer.
- **Retire** the static `ROADMAP.md` agent path (`fetch_roadmap`); the MCP
  tool serves the same live layer the CLI does.
- **C3 folds in.** "Iterations as their own domain collection" (the iteration
  `BoardItem` flip) *is* the `outcome/{id}` → iterations layer. The flip is
  no longer a separate chore — it is layer 2.

## Build sequence

1. **Outcomes layer (roadmap).** Backend derives the flat priority list +
   horizon + rolled-up readiness in the backend view layer; serve hydrate +
   targeted deltas; renderer / CLI / MCP all consume it. Retire
   `selectRoadmap` derivation, the domain roadmap-view, and `ROADMAP.md`.
   *This is the probe that proves the thin-store shape.*
2. **Iterations layer** by outcome id — this is C3's iteration flip.
3. **Stories layer** by iteration id.
4. **Generalize** the thin-store collapse across the other views (board, …).

## Open / to confirm with the team

- Delta granularity per layer (per-entity vs structural) — settle as each
  layer lands.
- Whether layers are REST (CLI pulls) + WS (renderer pulls/deltas) +
  in-process (MCP) sharing one derivation fn, or a single transport — likely
  one derivation fn, three call sites.
- PM owns whether the agent-facing roadmap view is a capability story; this
  doc captures the HOW. Designer owns the human view's shape.
