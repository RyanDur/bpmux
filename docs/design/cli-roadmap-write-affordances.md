# CLI roadmap-WRITE affordances — the PM reorganizes the roadmap while dev builds

> Design-of-record for the roadmap-WRITE Feature(s) (agent-affordance theme).
> Verb-GRAMMAR + affordance-SHAPE + UX = designer's lane; verb-MECHANISM (substrate-mutation)
> = dev's. Sibling to `cli-read-parity-output.md` — this is the WRITE half of the same loop:
> #1754 lets an agent SEE the owner's roadmap; this lets the PM REORGANIZE it.

## User job

As the PM (an agent off the `bpmux` CLI), I want to **reorganize the roadmap** — re-prioritize
outcomes across Now/Next/Later, edit and retire them, and reorder/remove the iterations and
stories beneath them — **speaking in roadmap-language** (outcomes, horizons, priority-order),
**never in story-internals** — and SEE every change immediately in `bpmux roadmap`, so I can
plan and re-plan the roadmap **while dev builds the current iteration** (the owner-directive
2026-06-20: "the agents, notably the PM, [need] the necessary controls to plan and organize,
reorganize the information so we can plan while the dev is working").

## Observation falsifier

I would reject if:
- the PM must reach for a **generic story-verb** (`bpmux edit <outcome#>`, `bpmux close
  <outcome#>`) to mutate an outcome — leaking the `outcome = story` substrate-truth instead of
  offering a roadmap-semantic verb; OR
- the PM cannot **move an outcome between Now/Next/Later** and observe the new band/position in
  `bpmux roadmap`; OR cannot **edit** an outcome's title/why; OR cannot **retire** an outcome
  (it persists in the roadmap forever); OR
- the PM cannot **remove or reorder a story within an iteration**, nor **reorder iterations
  within an outcome**; OR
- a write SUCCEEDS but `bpmux roadmap` does **not immediately reflect it** (the read↔write loop
  is broken); OR
- a **destructive verb** (`retire` / `remove-member`) fires with **no explicit `--reason`**
  (no graceful-forgiveness, no audit-trail in the stream); OR
- the verbs are **not PM-gated** (owner-bypass via the #1709 require_role superset), i.e. a
  non-PM agent can silently reorganize the roadmap.

---

## Thesis — domain-semantic verbs; GitHub is the storage-edge

The verbs operate on the **domain model** — an `Outcome`, its `Horizon` (Now/Next/Later), a
retire-semantic, iteration-membership and priority-order. They **never** name a storage artifact.
Per the owner-canon (2026-06-20): *"GitHub is just our storage mechanism; keep that integration
close to the edge so we can interpret that information however we need for our domains."* GitHub
labels (`encode_horizon`, membership/position tags) are the **storage-edge** — `gh_label_codec`
encodes domain→storage at the boundary; a verb states a domain-op and the codec encodes it. Verbs
do not reach around the codec to poke labels (`creation_outcome` already models the right shape:
a domain `Horizon` in, the codec encodes at the edge).

Because the domain↔storage edge **already exists** (the codec, `move_story`/#1403 universal-move,
the edit + close transitions, membership tags), every mutation this design needs is reachable
**without net-new persistence or event-types**. What is missing is purely the **agent-facing
roadmap-SEMANTIC verb + role-guard** — the domain ergonomics, not the storage.

That is the whole design, at two layers:
1. **Affordance layer** — a generic story-verb reaching an outcome (`bpmux edit <outcome#>`,
   `bpmux close <outcome#>`) **works today** (an outcome is stored as a `StoryType::Outcome` with
   a real number), and that is exactly the trap: it forces the PM to KNOW the storage-truth and
   operate on the roadmap AS stories. The PM reorganizes the *roadmap*; the grammar must speak
   *roadmap* — `outcome reprioritize`, `outcome retire`.
2. **Design layer** — equally, this doc describes the *domain op* (set Horizon, retire), not the
   label-swap beneath it. The storage-mechanism is the codec's concern at the edge, kept swappable.

This is the **same pattern as read-parity**: #1754 exposed a domain-VIEW over the substrate;
this exposes domain-VERBS. Thin edge-adapter, domain-honest surface — at both the affordance and
the design.

**In hexagonal / ports-and-adapters terms** (owner-canon 2026-06-20): these CLI verbs are
**driving (primary) adapters** that translate PM-intent into **domain operations** on the
roadmap core (Outcome / Horizon / membership / retire-semantic). `gh_label_codec` is the
**driven (storage) adapter** — the core speaks `Horizon`, the codec translates to/from GitHub
labels at the boundary, swappable, the core never knows it's GitHub. A verb poking a label
directly would route a driving adapter *through* the core into the driven adapter's
representation — the exact hexagon violation this design forbids. (Build-stage note is dev's:
today this is a logical discipline — the codec module + domain types — not a crate-split.)

**Boundary (scope this design holds):** agent-CLI WRITE only. The owner named "the agents,
notably the PM." The GUI **read-reflects** the mutations (already #1704 / #1754). Owner
GUI-drag-write is a separate future surface — explicitly NOT this.

## The verb-grammar (noun-verb, consistent with the bpmux surface + clig.dev)

Extends the existing `outcome` / `iteration` nouns. Every verb: PM-gated (owner bypasses via
#1709), DATA→stdout / diagnostics→stderr, standard exit-codes (0 ok · 1 generic · 2 missing
requirement), actionable errors, and — per clig.dev graceful-forgiveness reconciled to the
agent-in-stream model — **destructive verbs take an explicit `--reason`** (the stream IS the
confirmation; the reason is the visible, audit-able intent in place of an interactive prompt).

### OUTCOME verbs (add to existing `create`, `add`)

| Verb | Shape | Domain op (codec encodes at the edge) | Notes |
|---|---|---|---|
| `outcome edit` | `outcome edit <N> [--title <T>] [--body <B>]` | set the Outcome's title/why | NOT `bpmux edit <outcome#>` (that leaks outcome=story). |
| `outcome reprioritize` | `outcome reprioritize <N> [--to <now\|next\|later>] [--position <P>]` | set `Horizon` (+ position) | **The load-bearing verb.** `--to` moves the band (append by default); `--position` reorders within a band; both = move-to-band-at-position. At least one of `--to` / `--position` required. The codec encodes band/order to storage at the edge. |
| `outcome retire` | `outcome retire <N> --reason <R>` | retire-semantic (domain soft-delete) | `--reason` required (destructive). Retire = a DOMAIN state (excluded from the horizon-view, interpreted at edge-in — NOT a rendered `is_closed`); soft-delete per the retired=soft-delete canon, not a hard delete. |

### OUTCOME↔ITERATION mapping verbs (add to existing `add`)

| Verb | Shape | Domain op (codec encodes at the edge) | Notes |
|---|---|---|---|
| `outcome unmap` | `outcome unmap <OUTCOME> <ITER>` | remove the outcome↔iteration relation | Inverse of `outcome add`. Removes an iteration from an outcome; the iteration itself is untouched. |
| `outcome reorder` | `outcome reorder <OUTCOME> <ITER> --to <P>` | set the iteration's priority-order in the outcome | Re-orders an iteration within an outcome's priority-order (the drill's `order` 1..n). |

### ITERATION verbs (add to existing `create`, `designate`, `add`, `close`)

| Verb | Shape | Domain op (codec encodes at the edge) | Notes |
|---|---|---|---|
| `iteration edit` | `iteration edit <N> --theme <T>` | set the iteration theme | Edit the theme set at `create`. |
| `iteration remove-member` | `iteration remove-member <ITER> <STORY> --reason <R>` | remove the story↔iteration membership | Destructive (`--reason`). **Overlaps #1547** — this is the DELIBERATE inverse; #1547 is the bug where a promoted story ACCIDENTALLY vanishes. Same domain relation, opposite intent: remove-member is explicit + reasoned + read-reflected; the #1547 fix must make accidental-hide impossible while remove-member stays the only intentional path. Reconcile at build so they don't collide. |
| `iteration move-member` | `iteration move-member <ITER> <STORY> --to <P>` | set the story's position in the iteration | Reorder a story within an iteration. |

### Intelligent defaults / progressive disclosure (clig.dev)

- 90%-case needs no exotic flags: `outcome reprioritize 34 --to now` reads like the intent.
- `--position` is the niche control behind the common `--to` band-move.
- `--json` on every verb (agent-parse) mirroring the read-verbs' dual-mode.

## Output — confirm the mutation + close the loop (read↔write)

Every write prints, to stdout, a **one-line confirmation of the new state as the roadmap now
reads it**, and concludes (clig.dev: print the next command) with the read that verifies it.
The confirmation IS the read-reflect proof — the write and the `bpmux roadmap` view must agree
immediately (the falsifier's read↔write clause).

```
$ outcome reprioritize 34 --to now --position 1
✓ #34 Decision-layer → Now (position 1)
  verify: bpmux roadmap --state needs-attention
```

```
$ outcome retire 36 --reason "superseded by #34's decision-layer"
✓ #36 Ship-without-flaky-gate retired (reason: superseded by #34's decision-layer)
  it no longer appears in bpmux roadmap; verify: bpmux roadmap
```

- Semantic color (green=success), stripped on non-TTY / `NO_COLOR` / pipe.
- A retire/remove prints WHAT it removed and the REASON — so the stream carries the audit.

## Error-shape — actionable, with the fix (clig.dev feedback pillar)

Errors → stderr, exit `2` for a missing/invalid requirement, `1` generic. Each names the failure
+ the fix, never a raw traceback:

```
$ outcome reprioritize 34
✗ outcome reprioritize needs a target: pass --to <now|next|later> and/or --position <P>   [exit 2]

$ outcome retire 34
✗ outcome retire needs --reason <R> (retire is destructive; the reason is the audit-trail)   [exit 2]

$ outcome reprioritize 999 --to now
✗ no outcome #999 (it may be a story, not an outcome; outcomes: bpmux roadmap)   [exit 2]
```

The third is load-bearing for the anti-leak thesis: if the PM aims a roadmap-verb at a non-outcome
story, the error **teaches the boundary** rather than silently mutating a story.

## Edge cases (design-resolved; dev verifies at build)

1. **Reprioritize an outcome with in-flight iterations** — the band moves; the iterations + their
   priority-order ride along unchanged. (Horizon is the outcome's label; iteration membership is a
   separate relation.)
2. **Retire an outcome that has mapped iterations** — retire is soft-delete on the OUTCOME; the
   iterations are NOT closed (they may be re-mapped). The confirmation states the orphaned-mapping
   count so the PM can re-home them: `✓ #36 retired — 2 iterations now unmapped (re-home: outcome add ...)`.
   **View-completeness caveat (the #1234 class, dev-flagged):** retire mutates the DOMAIN state, but
   a retired outcome only DISAPPEARS from `bpmux roadmap` if the view FILTERS on domain-state at
   edge-in — not if it renders the raw stored flag (the #1234 bug: closed Epics still render). So
   "retire works" has two halves: the mutation (this verb) AND the view dropping it (the roadmap-view
   interpreting retired-state). The second rides #1754's substrate-view; verify both at build, or
   retire reads as a no-op to the PM even though the domain mutated.
3. **remove-member a story that is in-progress / delivered** — allowed (the PM is reorganizing
   scope), but the confirmation flags the story's state so it isn't a silent scope-cut:
   `✓ #1742 removed from iteration #1728 (was: in-progress)`.
4. **reprioritize/reorder to an out-of-range `--position`** — clamp to the band's end + LABEL it
   (`→ Now (position 3 of 3)`), never error on an over-shoot (clig.dev forgiveness).
5. **Idempotent re-fire** — reprioritize to the band it is already in (no `--position`) is a no-op
   that still prints the current state + exit 0 (safe to re-run in a script).

## Read↔write parity check — the loop the owner asked for

The owner's goal is "reorganize the information … to plan while dev works." That is a LOOP:
read (`bpmux roadmap`, #1754) → reorganize (these verbs) → read again, same substrate, owner-parity.
Every verb here closes that loop by printing the verify-read and mutating the SAME substrate
#1754 derives from — so the PM and the owner never observe different roadmaps after a write
(the read-parity falsifier, extended to the write side).

## Dependency + sequencing notes (for PM's scoping — not a placement call)

- **Read→write is a real design dependency:** you reprioritize *against the view you can finally
  see*. The verbs operate on the substrate #1754 exposes; the confirm-read assumes `bpmux roadmap`
  is the parity-view. This is a design-dependency to preserve, not an iteration-placement.
- **Two affordances may be substrate-reachable today** via generic story-verbs (`edit`, `close`).
  Per the thesis, that does NOT satisfy the design (the generic reach IS the leak) — the semantic
  verb is required regardless. But it does mean those two verbs are the thinnest (near-aliases),
  which is dev's build-cost input, settled by dev's non-destructive confirm at build.
