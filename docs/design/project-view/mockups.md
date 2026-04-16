# Project view mockups

*Visual targets for the three-zone layout from reshape.md. Each mockup
shows what Ryan sees given concrete data states.*

## Layout A: sidebar closed, happy path (~46 cols)

```
┌ project ────────────────────────────────────┐
│                                             │
│  ★ velocity (14d)                           │
│    delivered/day:  0.6                      │
│    pickup→deliver: 2.1h  (median)           │
│    deliver→accept: 4.8h  (median)           │
│    rework rate:    16%                      │
│                                             │
│  ★ dora (30d)                               │
│    shipped this week: 8                     │
│    in progress:       3                     │
│    cycle time:        3.2h  (avg)           │
│    rejection rate:    9%                    │
│                                             │
│  ★ board                                    │
│    READY (3)                                │
│      #224 Clickable peer names              │
│      #225 view_tokens MCP tool              │
│      #226 view_roadmap MCP tool             │
│    IN PROGRESS (1)                          │
│      #227 Project view reshape              │
│    DELIVERED (2)                            │
│      #220 Restart tool                      │
│      #219 Board poller                      │
│    ACCEPTED (47) | ICEBOX (8)               │
│                                             │
│  ⏱ updated 23s ago                          │
│                                             │
└─────────────────────────────────────────────┘
```

At 46 cols, both metrics labels and story titles fit comfortably.
Velocity + DORA use one-metric-per-row for readability.

## Layout B: sidebar open, cramped (~33 cols)

```
┌ project ─────────────────────────┐
│                                  │
│  ★ velocity (14d)                │
│    delivered/day:  0.6           │
│    pickup→deliver: 2.1h          │
│    deliver→accept: 4.8h          │
│    rework rate:    16%           │
│                                  │
│  ★ dora (30d)                    │
│    shipped this week: 8          │
│    in progress:       3          │
│    cycle time:        3.2h       │
│    rejection rate:    9%         │
│                                  │
│  ★ board                         │
│    READY (3)                     │
│      #224 Clickable peer…        │
│      #225 view_tokens MCP…       │
│      #226 view_roadmap MCP       │
│    IN PROGRESS (1)               │
│      #227 Project view reshape   │
│    DELIVERED (2)                 │
│      #220 Restart tool           │
│      #219 Board poller           │
│    ACCEPTED (47) | ICEBOX (8)    │
│                                  │
│  ⏱ updated 23s ago               │
│                                  │
└──────────────────────────────────┘
```

Story titles truncate with `…` at ~22 chars. Metrics still readable
at one-per-row. No attempt to fit two metrics per row at this width —
breaks alignment and legibility.

## Layout C: sparse data (new repo, < 3 completions)

```
┌ project ────────────────────────────────────┐
│                                             │
│  ★ velocity (14d)                           │
│    insufficient data (1/3 completed)        │
│                                             │
│  ★ dora (30d)                               │
│    shipped this week: 0                     │
│    in progress:       1                     │
│    cycle time:        insufficient data     │
│    rejection rate:    insufficient data     │
│                                             │
│  ★ board                                    │
│    READY (0)                                │
│    IN PROGRESS (1)                          │
│      #1 First story                         │
│    DELIVERED (0)                            │
│    ACCEPTED (0) | ICEBOX (0)                │
│                                             │
│  ⏱ updated 4s ago                           │
│                                             │
└─────────────────────────────────────────────┘
```

Sparse-data honesty per #218 and #209: don't show misleading numbers
on thin data. Counts are always real (0 is a real number for "shipped
this week"); medians/rates are suppressed.

## Layout D: auth missing

```
┌ project ────────────────────────────────────┐
│                                             │
│  ⚠ GITHUB_TOKEN not set                     │
│     — limited visibility; falling back to   │
│     manual board fetch via /board           │
│                                             │
│  ★ board                                    │
│    (use /board to refresh)                  │
│                                             │
│                                             │
│                                             │
│                                             │
└─────────────────────────────────────────────┘
```

Auth-missing state hides velocity/DORA (both require the poller) but
falls back to manual board via /board. Banner explains WHY and points
at the fallback. No velocity/DORA section at all — don't show empty
boxes for features that can't render.

## Layout E: network error after previous success

```
┌ project ────────────────────────────────────┐
│                                             │
│  ★ velocity (14d)                           │
│    delivered/day:  0.6                      │
│    pickup→deliver: 2.1h  (median)           │
│    deliver→accept: 4.8h  (median)           │
│    rework rate:    16%                      │
│                                             │
│  ★ dora (30d)                               │
│    shipped this week: 8                     │
│    in progress:       3                     │
│    cycle time:        3.2h  (avg)           │
│    rejection rate:    9%                    │
│                                             │
│  ★ board                                    │
│    READY (3)                                │
│      #224 Clickable peer names              │
│      #225 view_tokens MCP tool              │
│      #226 view_roadmap MCP tool             │
│    IN PROGRESS (1)                          │
│      #227 Project view reshape              │
│    ...                                      │
│                                             │
│  ⚠ offline — updated 2m ago                 │
│                                             │
└─────────────────────────────────────────────┘
```

Show last-successful snapshot with age. Banner flips from "⏱ updated"
to "⚠ offline — updated Xm ago." Data stays visible; user knows it's
stale. Never blank.

## Layout F: rate-limit state

```
┌ project ────────────────────────────────────┐
│                                             │
│  ...                                        │
│                                             │
│  ⚠ rate limit — retrying in 42s             │
│     last updated 1m ago                     │
│                                             │
└─────────────────────────────────────────────┘
```

Rate-limit differs from offline: we KNOW when we'll retry. Countdown
updates per frame. Data stays visible above.

## Styling notes

- `★` prefix: Dracula purple, bold — marks zone headers
- `⏱` prefix: Dracula comment (muted) — normal timestamp
- `⚠` prefix: Dracula orange — degradation or error state
- Metric labels: default foreground
- Metric values: Dracula cyan (data prominence)
- "insufficient data": Dracula comment (muted, honest absence)
- Story titles: default foreground
- State headers (READY, IN PROGRESS): colored per state (green,
  orange, yellow, comment) matching existing board rendering

## What these mockups commit us to

- Vertical stack: velocity → DORA → board → footer
- One-metric-per-row at both widths (simplifies layout math)
- Zone headers with `★` glyph for scannability
- Footer badge changes glyph + text based on state
- Never blank — always show stale data with age on error
- Auth-missing has its own layout (no empty velocity/DORA boxes)

What these mockups do NOT specify (dev's call):

- Exact Rust struct shapes or render functions
- Scroll behavior if board grows past panel height
- Interaction with the broader layout engine
