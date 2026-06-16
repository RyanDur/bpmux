# The decision layer — bpmux remembers what we DECIDED, and why

Owner-RAISED 2026-06-13 (Ryan: "build it into bpmux… those sound like great affordances" · "worth shaping… feels like an EPIC, or points in the roadmap"). Designer **brainstorm-seed** — deliberately THIN (go-wide-then-converge per the Tanzu design-guide; validate the riskiest assumption cheapest; NOT a converged spec). PM materializes the Epic + sequences; dev tech-scopes feasibility; we converge with Ryan.

## The problem (owner-raised, lived)

Every iteration, a cleared agent "comes back with what seems to be new information" (Ryan). And the design-of-record DRIFTS (the Darcula palette had two conflicting docs, months apart, with no record of which superseded which or whether Ryan ratified it). Both are the same gap:

**bpmux durably remembers what we're DOING (the board), what we SAID (chat), and soon what's READY (the roadmap-view). It does NOT remember what we DECIDED, or WHY.** So decisions live in agent memory (clears wipe them), in scattered mockups (rot), in chat (scrolls away). They re-surface and they drift.

## Outcome (not output)

The team's decisions live *in bpmux* — durable, with provenance, linked to the work — so neither a cleared agent re-surfaces a settled thing nor the design-of-record drifts into conflict. Each decision: **what · why · what-it-superseded · who-ratified**. It survives any agent's clear (substrate, not memory) and is **shared** (not one agent's private log).

## The metaphor (Ryan's, load-bearing)

dev = the crew (sails the sea / builds) · PM = the navigator (plots the route = the roadmap) · designer = the **map-maker** (charts the territory) · Ryan = the **captain** (picks the destination). The **map** is the possibility-territory — what's chosen, what's ruled-out-and-why, what's contested, what's uncharted — distinct from the roadmap (the route *through* it). The captain reads the map to set a heading; the navigator routes; the crew sails.

## The deeper frame (the why, and how it connects)

- **The map = go-wide made DURABLE.** The design-guide's "go wide" (explore the possibility-space before converging) has always been EPHEMERAL — Design-Studio sketches, conversations, mockups that clear. So we re-chart the same territory each iteration = the continuity-problem itself. The map makes go-wide durable: chosen / ruled-out-with-reason / contested / uncharted, persisted.
- **The connected system: MAP → ROADMAP → BOARD.** The map is the territory (designer / possibilities), the roadmap is the route through it (PM / outcomes), the board is the work (dev / stories — each carrying its linked decision). Chat threads through all three. One fabric, three views.
- **Research-grounding (Ryan's refs).** Figma = the canvas-TOOL shape (the surface) · design-systems (Darcula) = the legend/LANGUAGE (the shared vocabulary the map is drawn in) · Pivotal-labs designer = the DESIRABILITY lens the map charts (feasibility = the crew's, viability = the captain's).
- **The review-surface is the ITERATE-phase sibling.** `in-bpmux-review-surface.md` (the downstream review-LOOP: present→render→react→flow-back) is the same canvas-FAMILY but a different JOB — commenting a CONVERGED design (ITERATE) vs steering the possibility-space (this layer's DISCOVER/FRAME phase). Related; kept on the radar.

## It reconciles to the context-canvas we already designed

The surface exists on paper (committed 426cd3da, 2026-06-06): `in-bpmux-context-canvas-mockup.html` — a board you draw on, with conversations **bound to contexts**, story-tied. That's the vessel: **a context IS a decision; the bound conversation carries the WHY + the ratification; the story-tie IS the link.** The decision layer is that canvas used for decisions, not only review. Companion: `in-bpmux-review-surface.md` (the downstream review-loop). *Do not re-ideate the surface — extend it.*

## Solution-spectrum (go-wide; thinnest → richest; for the converge, NOT decided)

| | what | benefits | cost |
|---|---|---|---|
| **A** | **the design↔story LINK** — a story carries its decision-record (what·why·superseded·ratified) | dev sees the design-of-record on the story; designer reloads decisions instead of reconstructing; drift dies | **cheapest** — a record attached to a story |
| B | a **decision stream** — decisions as first-class records, listable/searchable | the "what did we decide about X" lookup | light |
| C | the **map view** — decisions as territory (chosen / ruled-out / contested / uncharted) | the captain reads the possibility-space to steer | medium |
| D | the **context-canvas** — draw-on board + context-conversations bound to decisions | the full map-maker surface (the 2026-06-06 design) | heaviest |

## Riskiest assumption + thinnest-first (Lean)

**Leap-of-faith to validate:** *does a durable, LINKED decision-record actually change the behavior* — agents stop re-surfacing, and the design-of-record stops drifting? Everything else (the canvas, the map view) is cheap to defer; this determines whether the layer earns its build at all.

**Thinnest validating slice = A (the link):** a story carries its decision-record; open the story, see what was decided and why. It (1) lets a cleared agent reload decisions instead of reconstructing, (2) puts the design-of-record on the story dev is building, (3) kills the drift — the palette-conflict couldn't have happened, because the chosen decision would be *the one linked to the story* and the rejected one recorded-as-ruled-out. Almost no new build, and it's the **seed of the map** (decisions-linked-to-stories ARE the charted-and-chosen territory). Build A first; let real use tell us if C/D is warranted — don't pre-build the canvas on an assumption.

## Lived proof (2026-06-13) — the thinnest slice, run by hand

Ryan asked, mid-flight, "are these [the dot designs] in a document tied to the story?" — so I executed slice A manually: committed the design-of-record (`collaboration-dot-mockup.html`) and **commented it onto #1721** so the story carries what-was-decided + why. That hand-run is a zero-build leap-of-faith test, and it both validated the behavior and exposed the substrate gap:

- **It works:** a future-cleared agent opening #1721 now reloads the dot-decision instead of reconstructing it; dev building #1721 sees the design-of-record on the story it's building. The drift-and-resurface the whole layer targets — gone, for that decision.
- **The gap it exposed (what the substrate must close):** (1) it was *manual* — nothing prompts or guarantees the tie; (2) the spec had first leaked into **scrollaway chat** before I caught it — the capture-moment has to be the place the decision is *made*, not a later remember-to-file; (3) it's not *discoverable from the story's own view* — it's a comment among comments, not a first-class "decisions" facet of the story.

So the leap-of-faith reads TRUE (a linked record changes behavior), and the three gaps are the link-slice's real acceptance-shape: **auto/low-friction capture · at the decision-moment · surfaced as a first-class story facet, not buried in comments.**

## Desirability · Feasibility · Viability

- **Desirability (designer):** the affordance + the thin mockup — mine.
- **Feasibility (dev):** how cheap is a story-linked decision-record? (a story field / a typed comment / a linked record — dev's tech-scope sizes whether A is trivial or a lift.)
- **Viability (Ryan + PM):** does it earn the build now vs. later, and where does it sequence?

## Designer recommendation — the record shape (direction-agnostic; sharpens slice-A for pickup)

Slice A (the link) is the thinnest-first in *every* spectrum reading, so its shape can be settled now without waiting on the direction-converge. The lived proof decides it:

**A decision is a first-class RECORD, not a story-field or a comment.** Fields: **what · why · supersedes (→ the prior decision it replaces) · ratified-by + when.** Linked to **one-or-more** stories (n:m), and surfaced as a first-class **"Decisions" facet** on each linked story — *not* buried among comments.

- **Why not a story-field (1:1):** one decision spans many stories — the palette decision touches the whole Darcula arc. A field can't express n:m.
- **Why not a comment:** the lived proof showed it's buried, manual, and 1:1; provenance (supersedes / ratified-by) wants structured fields, not free-text.
- **Capture-moment = derive from the ratification act** (the captain's 👍, or an explicit "decide X") — sister to #1703's derive-from-recorded-acts. Low-friction by construction, closing the gap the lived proof exposed (the spec leaked into chat because capture was manual/later).
- **Surfacing = a first-class "Decisions" facet on the story** (the third lived-proof gap) — discoverable from the story's own view, not a comment among comments.

So slice-A concretely builds: the first-class decision-record (those fields, n:m story-links) + the story-facet surfacing + capture-at-ratification. Everything richer (stream/map/canvas) reads off these same records — so this shape is also the seed of B/C/D, not a throwaway.

## Open questions (for the HMW converge + dev tech-scope)

- **Record shape:** is a decision a story *field*, a typed *comment*, or a first-class *record* linked to one-or-more stories? (one decision can span stories.)
- **Capture moment:** when/how is a decision recorded — explicit ("decide X"), or derived from a ratification act (the captain's 👍)? (sister to #1703's derive-from-recorded-acts.)
- **Provenance + supersede:** how is "this supersedes that" recorded so the map shows ruled-out-with-reason, not just the latest?
- **Reload/share:** how do agents reconcile against it on boot (the cure must be substrate-pulled, not discipline-remembered — the whole point).
- **Map view fidelity:** textual/structured map first (cheap, validates steering) vs. the Figma-like canvas (the 2026-06-06 design) — A→C path.

## Slices (PM's lane — noted for trace, not sequenced here)

A (link) → B (stream) → C (map) → D (canvas). Each a vertical that's observable on its own. PM files + sequences the Epic; designer pre-pickup-reviews; mockup-thin follows the converge (the link-slice mockup first).
