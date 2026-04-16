# Project view reshape

*What the project panel should show after #219 (polling), #218 (DORA
accuracy), and the slash-command audit. The project view is where
Ryan looks for "how's the team doing?" — make it useful.*

## Current state

Two render paths compete:

1. **With board data loaded** — story groups (Ready / In Progress /
   Delivered / Accepted)
2. **Fallback** — markdown roadmap/DORA content

#219 makes path 1 always-current via polling, so path 2's fallback
should rarely show. But the current layout has no room for velocity or
DORA alongside the board — they're separate renders entirely.

## Target

The project view is a single scannable summary with three zones,
top-to-bottom:

```
┌ project ─────────────────────────────────────────────────┐
│                                                          │
│  ★ velocity (last 14 days)                               │
│    delivered/day:    0.6     median pickup→deliver: 2.1h │
│    rework rate:      16%     median deliver→accept: 4.8h │
│                                                          │
│  ★ dora (last 30 days)                                   │
│    shipped this week: 8      avg cycle time:       3.2h  │
│    in progress:       3      rejection rate:        9%   │
│                                                          │
│  ★ board                                                 │
│    READY (3)                                             │
│      #221 Migrate story tools to octocrab                │
│      #223 Clickable channel names in sidebar             │
│      #224 Clickable peer names in sidebar                │
│    IN PROGRESS (1)                                       │
│      #222 trim rust-cache size                           │
│    DELIVERED (2)                                         │
│      #220 restart tool                                   │
│      #219 background board poller                        │
│    ACCEPTED (N) | ICEBOX (M)                             │
│                                                          │
│  ⏱ last updated: 23s ago                                 │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

## Zone rules

### Velocity (top, 4 numbers)

Four metrics on two lines. All derived from bpmux workflow signals
(#209 compute module). Numbers only — no labels on separate lines.

If sparse (< 3 deliveries in window): show `insufficient data (N/3)`
inline instead of each number. Match #209's honesty discipline.

### DORA (middle, 4 numbers)

Four metrics on two lines. Derived from board state + timestamps
(#218 compute module). Same sparse-data honesty.

### Board (main, scrollable)

Story groups by state. Same rendering as today:
- Active states (Ready, In Progress, Delivered) show story numbers +
  short titles, up to 5 per state with "+N more" overflow
- Accepted + Icebox compressed to count only

### Footer (timestamp)

`last updated: Xs ago` / `Xm ago` / `Xh ago` using #219's
`format_last_updated`. When fetch fails, show error state (network,
auth, rate-limit) from #219's user_message mapping.

## What gets deleted

- The fallback markdown path (`app.project.content` render at
  `ui.rs:500+`). If data is absent, show "initializing..." briefly
  until first poll completes. Don't fall back to static content.
- The separate DORA markdown rendering path. DORA numbers integrate
  into the zone above, not as a separate section.

## What stays

- The project panel exists/toggles the same way (side-by-side with
  chat via header click or #196's panel mechanism)
- Story rendering logic in `render_project_panel` — same function,
  richer content

## Error states (per #219's classification)

- **auth missing**: "⚠ GITHUB_TOKEN not set — limited visibility" at
  top of view, in place of velocity + DORA (which need the poller
  running). Board shows cached state if any.
- **rate limit**: timestamp shows "retrying in Xs — last updated Xm
  ago"
- **network**: timestamp shows "offline — last updated Xm ago"
- **other**: timestamp shows "error: brief message — last updated Xm
  ago"

Never blank. Always show stale-with-age if fresh fetch fails.

## Layout constraints

- 80-column TUI — zones stack vertically, numbers align on their own
  line
- When project panel is in split-view mode (60/40 with chat):
  - **Sidebar closed**: content area is ~78 cols; project panel at 60%
    is ~46 cols usable. All three zones fit readably.
  - **Sidebar open**: content area is ~56 cols; project panel at 60%
    is ~33 cols usable. Velocity/DORA two-column rows are tight. Board
    story titles truncate more aggressively.
- Velocity and DORA rows use two metrics per row (`A: N  B: M`) to
  fit narrow widths. If 33-col rendering breaks legibility, fall back
  to one metric per row.

## What this spec does NOT cover

- Implementation details (which struct holds what, which module
  renders which zone) — dev's call
- Color/glyph choices beyond what the existing board view uses —
  keep consistent with current theme
- Agent-triggered expansion (clicking a zone for more detail) —
  separate story if needed
- Historical trend lines / charts — future if the basic numbers
  don't tell the story

## Next steps for PM

Two stories this spec implies:

1. **Project view reshape**: single view with velocity + DORA + board
   zones. Deletes the markdown fallback. Requires #219 (shipped).
2. **Error-state polish**: auth-missing banner, network/rate-limit
   timestamps. Depends on #219's error classifier.

Both could ship as one story if small. Dev's call whether to split.
