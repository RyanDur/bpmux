# Planning levels: outcome · epic · iteration · story

The four units we plan with, and the **two axes** they live on. The
recurring confusion ("is this an epic or an iteration?") dissolves once
you see they answer different questions:

- **The value axis (the *what / why*)** — vertical: `outcome → epic → story`.
  How we decompose what we're building and the value it serves.
- **The commitment axis (the *when*)** — horizontal: `iteration`.
  Which work the team commits to drive to done *now*. Cuts across the
  value axis.

A single story sits on **both** axes at once: it serves one outcome
(its epic) and is driven in one cycle (its iteration). These are
independent — changing one doesn't change the other.

---

## The four units

### Story — the unit of work
One capability, end-to-end (a vertical slice), with a Use-Case + an
observation-falsifier. The thing that actually gets built, reviewed,
and accepted. (Format canon: `story-shape.md`.)

### Epic — a body of work, grouped by capability
A large capability that spans multiple stories — the **container** for
related work. No time-boundary: an epic is "done" when its stories are
done, however many cycles that takes. Answers *"what big thing does
this story serve?"*
- **bpmux substrate:** the parent-epic link. An epic-card (e.g. #1722)
  with child stories (#1723–1727).
- **Named by outcome, plain language** (`feedback_epic_names_by_outcome_plain_concise`).

### Outcome — the destination, on the roadmap
The change in the world we're steering toward — the *goal*, measurable,
viewed over time with readiness. Where the epic is the **work-container**,
the outcome is that work seen as a **destination**. In current practice
they're near-1:1 (each epic named by its outcome); conceptually an
outcome is the *why* and can be served by more than one epic over time.
- **bpmux substrate:** rendered by the roadmap-view (#1704/#1702) —
  outcomes in order, each with rolled-up readiness, plain names never
  "Epic #N".
- Distinct from the **roadmap** itself: the roadmap is the *route* through
  the outcomes (the navigator's plot); the outcome is a *waypoint* on it.

### Iteration — a commitment-cycle
The set of stories the team commits to drive to terminal-state
**together, now**. Scope-boxed, not time-boxed: it closes when its scope
completes, with no defer/carry-over
(`feedback_iteration_scope_commits_team_until_next`). It **cuts across**
epics — one cycle mixes a bug-fix, a Darcula slice, a decision-layer
slice.
- **bpmux substrate:** an iteration entity + story membership
  (e.g. Crux #1707, holding #1719/#1720/#1721).

---

## The orthogonality (the key relationship)

- **One epic's stories spread across MANY iterations.** The decision-layer
  epic ships slice A this cycle; B/C/D in later cycles. The epic outlives
  any one iteration.
- **One iteration contains stories from MANY epics.** A single cycle pulls
  the most valuable next stories regardless of which outcome they serve.

So for any story, two independent questions:
- *Which outcome does it serve?* → its **epic** (value axis)
- *Which cycle are we driving it in?* → its **iteration** (commitment axis)

Analogy: an **epic** is a themed playlist (all the "decision-layer"
tracks); an **iteration** is tonight's setlist (what we actually play
this show, pulled from several playlists). The same track lives in one
playlist but appears in whichever show you slot it into.

---

## How each shows up in the work

| Unit | Question | Lifespan | Owner of the call | Substrate |
|---|---|---|---|---|
| Outcome | what change are we steering to? | many cycles | Ryan (captain) sets; PM frames | roadmap-view |
| Epic | what body of work serves it? | many cycles | PM decomposes | parent-epic link |
| Iteration | what do we commit to now? | one cycle | PM composes; team commits | iteration entity |
| Story | what capability, end-to-end? | one pickup | PM/designer shape; dev builds | the card |

Planning flows **down the value axis** (outcome → epic → enumerate the
full vertical-slice story-set) and is **composed on the commitment axis**
(pull the next stories across epics into a coherent, theme-sized cycle).
Decomposing an epic means capturing its *entire* slice-spectrum as
stories (filing ≠ building — capture preserves the ideas; build-order
stays thinnest-first). See `iteration-ceremony-cycle.md` for the
close→retro→plan→open cadence and the 3-iteration horizon.

## Related canon
- `story-shape.md` — the story unit's format.
- `iteration-ceremony-cycle.md` — iteration cadence + horizon.
- `process-state-machine.md` — the states a story moves through.
