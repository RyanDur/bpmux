# Reject-context as first-class — design

Designer-owned, 2026-06-06. Ryan-raised pain (verbatim): "when I reject a story in an
iteration and it goes into the next, its rejection gets lost and you default to the base
story… and you seem not be able to read the comments for the rejection. this is a pain and a
worry… it is a major pain, and makes it hard for us to work together, or for you to work
autonomously." Priority: ASAP (owner).

Substrate/hydration = dev. Board-filing/sequencing = PM. This doc owns the **design**: what the
reject-context IS, and how/where it surfaces.

## Principle — a reject is the rework-SPEC, not a comment

A reject is the single most load-bearing piece of context for a rejected story — literally
*what to fix*. Today it lives in a gate-comment that:
- **(a) tools return UNHYDRATED** — `get_story` / `view_comments` / `view_story_timeline` all
  come back `gate_comments_hydrated: false` (verified firsthand 2026-06-06), so no agent can
  read the rejection_reason at-source; only Ryan's manual paste surfaces it.
- **(b) is orphaned on iteration-move** — when a rejected story moves to the next iteration,
  the reject doesn't ride with it; pickup defaults to the **base** spec.

Result: rework grounds on the stale base, not the reject → the paraphrase-drift that failed
Aquila (4/4) and cost ≥2 manual pastes today (#1524, #1517). **The IA failure: the MOST
important context is the LEAST accessible.** The cure is to elevate the reject to first-class
state — readable at-source, carried with the story, and surfaced at the moment drift happens
(pickup).

## Three surfaces

### 1 · READ at-source — kills the manual paste
`get_story` returns a first-class field:
```
latest_reject: { reason (VERBATIM — never paraphrased), by, at, against_commit, gate }
```
Hydrated — never `gate_comments_hydrated: false` for the reject. `view_story_timeline` surfaces
every reject event hydrated too. Any agent reads Ryan's exact words from the tool; the manual
paste is no longer load-bearing. *(Substrate: dev hydrates. Design: the reject is a named
first-class field, not buried in an opaque comment-stream.)*

### 2 · CARRY across iteration-move — kills default-to-base
`latest_reject` is persistent story-state, not a comment that orphans. When a rejected story
moves to the next iteration it rides along. The story's **effective rework-spec = base-spec +
the active reject** (the reject is the delta-to-close). It clears only when the story re-passes;
a new reject replaces the prior.

### 3 · SURFACE at pickup + in the view — kills the drift
The drift happens at **pickup** (you ground on the base, miss the reject). So surface it there,
first:

```
  start_story #1517 →
  ┌──────────────────────────────────────────────────────────┐
  │ ⚠ REWORK — rejected by ryan · 2026-06-06 09:01 · @eff0626a │
  │   "drag sort does not persist and does not have the same   │
  │    drag css as the iteration"                              │
  ├──────────────────────────────────────────────────────────┤
  │ (then the base spec / user-job / falsifier follow)         │
  └──────────────────────────────────────────────────────────┘
```
The first thing grounded-on is *what to fix*, in Ryan's exact words — not the base spec.

And in the bpmux board/story-view (Ryan's + agents' glance): a rejected story carries a
**reject-badge**; the reason reveals on expand. Ryan sees WHY at a glance; agents read it
without digging. (Ties to the now-tab/board-IA surface; same "load-bearing context is
glanceable" principle.)

## Facet — the loss corrupts gate-STATE, not just context (lived 2026-06-07, #1524)

PM + designer hit this firsthand re-desk-checking #1524. The story carried a STALE gate-state
from its Aquila reject that **never reopened**: `get_story` returned `workflow_state: accepted`
+ `is_closed: true` + a stray `in progress` label, while `view_story_timeline` returned ONLY a
`filed` event (every reject/deliver/desk-check marker `gate_comments_hydrated: false`) and the
`desk_check` + `delivered` columns were both empty. The substrate said "accepted+closed"; the
truth was "rework never reopened, desk-check never ran." It nearly drove a `pass_desk_check` on
a closed story (caught by grounding the authoritative column + timeline, not `get_story`).

So the reject-record loss has a **STATE dimension on top of the context dimension**: an
unhydrated gate-record doesn't just hide the *reason* — it leaves `workflow_state` **mis-derived**
(a closed reject reads as an *acceptance*), so the state-machine position itself is wrong. The
same hydration that surfaces `latest_reject` (surface 1) MUST also make the gate-records
**authoritative for state-derivation**, so `workflow_state` follows the real last gate-transition
(reject → ready/rework), never a stale GitHub closed-state. Hydrating the reason but leaving the
state mis-derived would still strand the story in a phantom-accepted limbo. *(Substrate: dev. The
design point: the reject-record is load-bearing for the STATE machine, not only the rework-spec.)*

### FACET 3 (#1691) — the state-derivation observation-falsifier

The state-derivation fix (its own story, depends on #1686's hydration feeding it) has a
load-bearing falsifier of its own. I would reject if I observed ANY of:

1. **Phantom-accepted (core)** — a carried-reject story reads `workflow_state` =
   accepted/closed/any terminal NOT derived from its real last gate-transition; the derivation
   must yield `ready`/`rework`.
2. **Source-precedence inverted** — the derivation reads the stale GitHub closed-state (or raw
   `is_closed`) instead of the hydrated gate-records' last gate-transition.
3. **Hydration without derivation (the half-fix)** — gate-records are hydrated (#1686) but
   `workflow_state` still mis-derives because the derivation isn't wired to them.
4. **Backward-transition blindness — THE CRUX (= the #1524 mechanism)** — the derivation reads
   the MOST-ADVANCED / forward-only gate and misses the backward reject (a reject moves the story
   BACKWARD: desk_check/delivered → ready). It must read the AUTHORITATIVE last transition BY
   TIMESTAMP; a most-advanced / dual-label decode silently drops the backward reject. (Sister to
   the banked "probe-backward transitions for completeness" lesson — the same most-advanced-decoder
   trap.)
5. **Over-correction on a real accept (inverse failure)** — a story whose real last transition is
   a genuine accept mis-derives to rework; the derivation must distinguish a real accept-transition
   from a reject-then-closed (a normally-accepted, non-carried story is unaffected).
6. **Vacuous coverage** — the load-bearing test seeds a carried-reject fixture (hydrated
   gate-records whose LAST transition by-time = reject + a STALE closed/accepted source) and asserts
   `derive → ready/rework` (RED without the derivation). Omitting the accept-discrimination case
   (genuinely-accepted fixture → stays accepted) is VACUOUS for #5; not seeding the backward reject
   as last-by-time can't catch #4.

The crux is #4: read the authoritative last gate-transition by timestamp, discriminating a
backward reject from a forward accept — never the stale GitHub closed-state.

## Observation-falsifier (the design's AC)

I would reject if:
1. `get_story` returns the reject UNHYDRATED — can't read it at-source (the paste is still
   required).
2. the reject is LOST / defaults-to-base when the story moves to the next iteration.
3. the pickup-context does NOT surface the reject FIRST — base-spec leads, reject buried below
   or absent.
4. the reject in the board/story-view needs digging (not glanceable — no badge, or hidden in an
   unhydrated comment-stream).
5. the verbatim reject is PARAPHRASED or lossy anywhere in the chain — the at-source word is
   canonical; a summary is the exact failure mode that dropped #1524's sync-clause.
6. a rejected-then-carried story's `workflow_state` reads `accepted` (or any non-rework terminal)
   because its gate-records are unhydrated — the derived state must reflect the real last
   gate-transition (reject → ready/rework), never a stale closed-state (the #1524 corruption).

## Why first-class (not "just hydrate the comment")

Hydrating the read-tool (surface 1) stops today's paste-pain — pull it first. But the durable
cure is treating the reject as **rework-state**: carried (2) + surfaced-first (3). A hydrated-
but-still-buried, still-orphaned comment would read fine and still drift. The reject must be the
thing you can't miss at pickup — because missing it is the failure we keep paying for.
