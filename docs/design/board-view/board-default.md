# Board view: default — glanceable project state

*Opened via command palette (Ctrl+K → "board") or keybind. Shows all stories grouped by state.*

```
┌ board ───────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  READY (2)              IN PROGRESS (1)       DELIVERED (5)                  │
│  ─────────              ──────────────        ──────────────                 │
│  #140 bridge stories    #191 topic channels   #190 identity layer            │
│  #168 session notes                           #192 command palette           │
│                                               #180 sticky scroll             │
│                                               #185 bridge render fix         │
│                                               #186 filter widen              │
│                                                                              │
│  ACCEPTED (20)          DEFERRED (1)          ICEBOX (29)                    │
│  ────────────           ─────────             ──────────                     │
│  #132 #142 #145 ...     #148 P2P (tue)        29 stories                    │
│  (20 stories)                                                                │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
┌ ryan > ──────────────────────────────────────────────────────────────────────┐
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
 board view | Esc=back to chat                     ⌥1=topics Ctrl+K=actions
```

## Visual notes

- **Columns by state**: Ready, In Progress, Delivered, Accepted, Deferred, Icebox
- **Counts in headers**: "(2)" "(1)" "(5)" — glanceable without reading details
- **Story entries**: `#NNN short-title` — one line per story, enough to identify
- **Accepted + Icebox compressed**: large counts show total, not every story (20+ items would overflow). click/drill to expand.
- **Deferred has a reason**: "#148 P2P (tue)" — shows WHY it's deferred
- **One screen, no scrolling needed**: the board fits in the 80x24 viewport by compressing large categories

## Colors (Dracula palette)

- Column headers: DRACULA_CYAN, bold
- Ready stories: DRACULA_GREEN (ready to pull)
- In Progress: DRACULA_ORANGE (active work)
- Delivered: DRACULA_YELLOW (awaiting acceptance)
- Accepted: DRACULA_COMMENT (dim — done, not active)
- Deferred: DRACULA_PURPLE (parked with reason)
- Icebox: DRACULA_COMMENT (dim)
- Story numbers: DRACULA_FG
- Story titles: DRACULA_FG

## Interaction

- Arrow keys navigate between stories
- Enter on a story opens detail view (see board-details.md)
- Esc returns to chat
- The board refreshes on open (pulls current gh issue state)
