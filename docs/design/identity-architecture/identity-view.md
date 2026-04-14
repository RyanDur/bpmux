# Identity view — how identity surfaces in the TUI

*Opened via command palette (Ctrl+K → "identity") or /identity. Shows values, principles, and the arcs that shaped them.*

## Default view: values + principles

```
┌ identity: designer ──────────────────────────────────────────────────────────┐
│                                                                              │
│  CLARITY                              HONESTY                                │
│  ────────                             ─────────                              │
│  mockups as contracts                 verify through observation             │
│  behavioral constraints only          concerns push, opinions hold           │
│                                                                              │
│  CRAFT                                INDIVIDUALITY                          │
│  ───────                              ──────────────                         │
│  work outside-in                      learn through own lens                 │
│  two-audience + negative tests        peer feedback as information           │
│                                                                              │
│  ──────────────────────────────────────────────────────────────              │
│  recent arcs: 3    lessons: 12                                               │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
┌ designer > ──────────────────────────────────────────────────────────────────┐
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
 identity view | Enter=drill ↑↓=nav Esc=back                        Enter=send
```

## Drill-down: principle → arcs that shaped it

*User selects "two-audience + negative tests" and presses Enter*

```
┌ principle: two-audience + negative tests ────────────────────────────────────┐
│                                                                              │
│  Serves: CRAFT                                                               │
│  "every design considers both human AND agent experience.                    │
│   desk-check includes negative tests per audience."                          │
│                                                                              │
│  ARCS THAT SHAPED THIS PRINCIPLE:                                            │
│  ──────────────────────────────────                                          │
│                                                                              │
│  ▸ #191 topic channels saga                              reinforced 3x      │
│    display-only → rejected → delivery-side → accepted                        │
│                                                                              │
│  ▸ #193 board view                                       reinforced 1x      │
│    missing agent view_board → subagent caught it                             │
│                                                                              │
│  ▸ #196 board-in-panel                                   reinforced 1x      │
│    headless desk-check broken → couldn't verify                              │
│                                                                              │
│  Total reinforcements: 5 across 3 arcs                                       │
│                                                                              │
│  GUIDED ACTION IN:                                                           │
│  ──────────────────                                                          │
│  ▸ #193 board view — caught missing agent audience during desk-check         │
│  ▸ #196 board-in-panel — applied negative test (headless capture check)      │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
┌ designer > ──────────────────────────────────────────────────────────────────┐
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
 principle detail | Enter=open arc ↑↓=nav Esc=back                   Enter=send
```

## Deep drill: arc detail

*User selects "#191 topic channels saga" and presses Enter*

```
┌ arc: #191 topic channels saga ───────────────────────────────────────────────┐
│                                                                              │
│  Beginning: "context coherence is broken" (2026-04-12)                       │
│                                                                              │
│  ├─ Middle A: display-side built                                             │
│  │  └─ rejected: agents couldn't participate                                 │
│  │     → learned: two-audience coverage                                      │
│  │     → learned: negative desk-checks                                       │
│  │                                                                           │
│  ├─ Middle B: delivery-side wired                                            │
│  │  └─ desk-check passed (positive + negative)                               │
│  │     → learned: per-user REST enables verification                         │
│  │                                                                           │
│  └─ Middle C: live test with ryan                                            │
│     └─ auto-subscribe gap found                                              │
│        → learned: test the workflow, not the mechanism                        │
│                                                                              │
│  Documents: session log 2026-04-12, commits c2911bb..4a7c8cd                 │
│  Principles informed: two-audience, verify through observation               │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
 arc detail | Esc=back to principle                                  Enter=send
```

## Progressive disclosure in action

```
SHALLOW                    MEDIUM                       DEEP

/identity                  Enter on principle            Enter on arc
  values + principles        which arcs shaped it          branching narrative
  at a glance                + reinforcement counts        with learnings per end
```

## What agents see (via MCP)

`get_identity` returns:
```json
{
  "values": { "clarity": "...", "honesty": "...", "craft": "...", "individuality": "..." },
  "principles": [ { "name": "...", "serves": "...", "reinforcements": 5, "arc_refs": ["#191", "#193", "#196"] } ]
}
```

`get_arcs` (future tool) returns:
```json
{
  "arcs": [ {
    "id": "#191-saga",
    "summary": "topic channels: display-only → rejected → delivery-side → accepted",
    "branches": [ { "middle": "display-side built", "ends": [...], "learnings": [...] } ],
    "principles_informed": ["two-audience", "verify-through-observation"],
    "refs": { "session": "2026-04-12", "commits": ["c2911bb", "4a7c8cd"] }
  } ]
}
```

## Colors (Dracula palette)

- Value headers: DRACULA_CYAN, bold
- Principle names: DRACULA_FG
- Arc summaries: DRACULA_FG
- Reinforcement counts: DRACULA_GREEN (positive signal)
- Branch markers (├─ └─): DRACULA_COMMENT (structural, dim)
- Learnings: DRACULA_PURPLE (distinct from the arc narrative)
- Document references: DRACULA_COMMENT (dim, reference info)
