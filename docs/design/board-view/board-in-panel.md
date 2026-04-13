# Board in project panel — overlay that preserves chat

*Replaces the full-screen /board modal with board data inside the existing project panel.*

## Problem

The current /board takes over the full screen. Ryan can't chat while viewing project state. The project panel already exists as an overlay that preserves chat — putting board data there lets ryan see stories AND talk at the same time.

## Mockup: board data in project panel (panel open)

```
┌ ▾ topics ────────────┐┌ project ────────────────────────────────┐
│ ● general            ││ READY (0)   IN PROGRESS (0)             │
│   teaching           ││                                          │
│                      ││ DELIVERED (2)                             │
│                      ││ #191 topic channels                      │
│                      ││ #148 P2P connect                         │
│                      ││                                          │
│                      ││ ACCEPTED (164)  ICEBOX (29)              │
│                      ││                                          │
│                      │├──────────────────────────────────────────┤
│                      ││ ryan: go ahead design                    │
│                      ││                                          │
│                      ││ designer: starting board-in-panel        │
│                      ││           mockup now.                    │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ ryan > ─────────────────────────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 general | project open                        Esc=close Enter=send
```

## What this shows

- Project panel takes the TOP portion of the chat area (split view)
- Chat messages visible BELOW the panel — ryan can read AND see project state
- Input box stays at the bottom — ryan can type while the panel is open
- Panel is compact: only shows counts + active stories (delivered, ready, in progress). accepted + icebox compressed to counts only.
- Esc closes the panel, restoring full chat view
- Click/Enter on a story in the panel could drill into detail (future)

## How it differs from the full-screen /board

| Full-screen /board | Board-in-panel |
|---|---|
| Takes over entire screen | Shares screen with chat |
| Can't type while viewing | Input box stays available |
| Shows all 6 columns spread out | Compact: only actionable states visible |
| Needs Esc to return to chat | Chat is always visible below |
| Opened via /board or Ctrl+K | Opened via clicking the header bar (existing behavior) or Ctrl+K |

## Colors (Dracula palette)

- Panel border: DRACULA_SELECTION (subtle separation from chat)
- State headers: DRACULA_CYAN, bold
- Story numbers: DRACULA_FG
- Counts for compressed categories: DRACULA_COMMENT (dim)
- Divider between panel and chat: DRACULA_SELECTION (horizontal line)

## Interaction

- **Open**: click the header bar (already toggles project view) or Ctrl+K → "board"
- **Close**: Esc or click header again
- **Drill into story**: arrow keys to select, Enter to open detail (future slice)
- **Panel height**: adapts to content — more stories = taller panel, fewer = shorter. minimum: 4 rows (headers + one story). maximum: half the viewport (so chat always has space)

## What this does NOT change

- The full-screen /board still exists as a command — it becomes the "expanded view" for when you want the full column layout
- The panel is the "quick glance" view for everyday use
- The MCP `view_board` tool returns the same data regardless of how ryan views it
