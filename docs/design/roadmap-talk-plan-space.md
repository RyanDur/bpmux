# The roadmap as a plan + talk space — PM's planning surface, scoped so it doesn't disturb dev

Owner-DIRECTED 2026-06-14 (Ryan, verbatim): "you are the main user of the roadmap. it should be a place for you to plan what we have" · "it has to work for you, and be a place we can talk and plan while an iteration is happening" · "so we do not disturb the dev." Designer **brainstorm-seed** — go-wide-then-converge; thinnest-first. PM materializes; dev tech-scopes; we converge with Ryan. Future-arc (the talk/plan extension); the VIEW half rides Crux #1704/#1729.

## The problem (owner-raised)

Planning, and talking-about-the-plan, has nowhere scoped to live. Today it happens in **chat** (scrolls away — the same rot the decision-layer fights) or in **broadcast** (which pulls dev off their in-flight work — the exact "don't disturb the dev" pain Ryan named, and the live cost he caught this session when broadcasting roadmap-shaping dragged dev off #1720). PM is the roadmap's **main user** (owns building-the-right-thing-at-the-right-time), but the roadmap-view (#1704/#1729) is a *read* surface — there's no place to *plan + talk* on it, scoped to the planners.

## Outcome (not output)

The roadmap is **PM's planning surface** + a **bound plan/talk space** scoped to PM + designer + Ryan (dev EXCLUDED by construction) — so the people steering the route plan and talk *where they see the work*, during an iteration, without disturbing the crew. Durable (not scrollaway), bound to the roadmap (not a free-floating channel), participant-scoped (the in-app version of the `to:[designer,pm]` design-room).

## The family (load-bearing — this is NOT a new surface-class)

The PLANNING-phase sibling of two surfaces already shaped:
- **context-canvas** (`in-bpmux-context-canvas-mockup.html`, 2026-06-06) = talk bound to a **design**, in the DESIGN-REVIEW phase.
- **decision-layer** (#1722) = the record bound to a **story/decision**, in the DECISION phase.
- **roadmap plan/talk space** (this) = talk + planning bound to the **roadmap** (outcomes/iterations), in the PLANNING phase.

All three are the same shape: **talk + work bound together, participant-scoped.** Don't re-ideate the surface-family — extend it. The metaphor holds: the map (designer / territory) → the roadmap (PM / the route *and* where PM plans+talks it) → the board (dev / the work). Chat threads through all; the roadmap's talk is scoped to exclude dev.

## Solution-spectrum (go-wide; thinnest → richest; for the converge, NOT decided)

| | what | benefits | cost |
|---|---|---|---|
| **A** | the roadmap **VIEW** (#1704 owner-view / #1729 PM-CLI) — substrate-derived outcomes→iterations + readiness | PM + Ryan READ the plan from one truth (the Crux work) | in-flight (Crux) |
| **B** | a **bound plan/talk thread** on the roadmap — PM + designer + Ryan converse on an outcome/iteration, dev-excluded, durable | plan + talk *where you see the work*, no scrollaway, no dev-disturb | light–medium |
| **C** | the **plan-canvas** — draw-on/arrange the possibility-space (the map-maker surface), conversations bound to outcomes | the full steer-the-territory surface (the 2026-06-06 canvas, planning-applied) | heaviest |

## Riskiest assumption + thinnest-first (Lean)

**Leap-of-faith:** does a *scoped, bound, durable* plan/talk space actually change how we plan — replacing chat-that-scrolls + broadcast-that-disturbs-dev — or do we just keep talking in chat? Everything richer (the canvas) is cheap to defer; this determines whether the talk-layer earns its build.

**Thinnest validating slice = B (the bound plan/talk thread)** on top of A (the view, already in Crux): a thread bound to an outcome/iteration, scoped to PM+designer+Ryan (reusing the existing channel participant-scoping — the `to:[designer,pm]` filter, dev-excluded), durable. It (1) gives PM a place to plan+talk on the roadmap, (2) keeps it off dev's stream by construction (the don't-disturb-dev cure, mechanized not disciplined), (3) doesn't scroll away. Build B on A; let real use tell us if C (the canvas) is warranted — don't pre-build the canvas on an assumption.

## Desirability · Feasibility · Viability

- **Desirability (designer):** the surface + the thin mockup — mine (shaped with the canvas/decision-layer family so they cohere).
- **Feasibility (dev):** reuse the existing channel participant-scoping (the dev-exclusion is the same filter) + the #1704 roadmap-derive (the binding-target). dev tech-scopes how a thread binds to an outcome/iteration.
- **Viability (Ryan + PM):** earns the build vs. later, and where it sequences. PM is the primary user (it has to work for PM).

## Open questions (for the converge + dev tech-scope)

- **Reuse vs new:** is the plan/talk space the **context-canvas reused** (the canvas-family, one surface, three phases) or a **roadmap-specific thread**? (Lean: the same family — one surface, bound to different work per phase.)
- **Binding granularity:** a thread per OUTCOME, per ITERATION, or both (an outcome-thread + iteration-threads beneath)?
- **Scoping:** the dev-exclusion = the existing `to:[designer,pm]` participant-filter applied to the bound thread — confirm it composes with the roadmap-binding (the don't-disturb-dev is *by construction*, not discipline).
- **Durability + reload:** the plan-talk survives clears + is shared (sister to the decision-layer's substrate-pull, not memory).
- **Relation to the decision-layer:** a plan-talk that converges → a DECISION should flow into the decision-layer (#1722) — the planning-phase feeds the decision-phase. One fabric.
