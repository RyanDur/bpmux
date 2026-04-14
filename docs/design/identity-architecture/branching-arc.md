# Branching arc structure

*An arc is a tree of experience segments that produces multiple learnings.*

## The arc as a tree (not a line)

```
                        ┌─ End A1: rejected ──────── Learning: two-audience coverage
                        │                            Learning: negative desk-checks
    ┌── Middle A ───────┤
    │   display-side    └─ End A2: desk-check ────── Learning: display ≠ delivery
    │   built               missed delivery
    │
    │                   ┌─ End B1: auto-subscribe ── Learning: test workflow not mechanism
Beginning ──────────────┤   gap found
#191 filed              │
"context coherence      ├── Middle B ───────┤
 is broken"             │   delivery-side   └─ End B2: desk-check ────── Learning: per-user REST
                        │   wired               with negative test         endpoint enables verification
                        │
                        │                   ┌─ End C1: ryan can't ────── Learning: "not a feature
                        └── Middle C ───────┤   see agents              until both audiences work"
                            live test       │
                            with ryan       └─ End C2: topic routing ── Learning: sticky routing
                                                traps agents             needs structural fix
```

## How to read this

- **Beginning**: the event that starts the arc (a problem named, a story filed)
- **Middles**: attempts to address the beginning. each middle is a different approach.
- **Ends**: outcomes of each middle. each end produces one or more learnings.
- **Learnings**: specific rules extracted from the end (what we now know)
- **Branches**: the arc SPLITS at decision points. "we tried A, which led to X and Y. we also tried B, which led to Z."

## One beginning, many learnings

The #191 arc produced at least 8 distinct learnings from 3 middles and 6 ends. A linear model (beginning → middle → end → one learning) would lose this richness. The tree preserves it.

## Learnings can share arcs

Multiple arcs can reinforce the SAME learning. "two-audience coverage" was learned from #191 AND from #193 (board view missing agent audience). When the same learning appears in multiple arc trees, it's a candidate for promotion to a principle.

```
Arc: #191 ───── Learning: two-audience coverage
                         ▲
Arc: #193 ───── Learning: two-audience coverage ──── reinforced ──→ Principle
                         ▲
Arc: #196 ───── Learning: two-audience coverage
```
