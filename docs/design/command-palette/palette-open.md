# Command palette: open state

*User presses Ctrl+K to open the palette. Types partial text to filter actions.*

## Empty state (just opened)

```
┌ topics ──────────────┐┌ teaching protocol ───────────────────────┐
│ ● teaching protocol  ││                                          │
│   scroll fix  (3)    ││ ryan: lets talk teaching                 │
│   general            ││                                          │
│                      ││ pm: two-shaped currency...               │
│                      ││                                          │
│ ┌ actions ─────────┐ ││ designer: that reframes our proposal...  │
│ │ > _              │ ││                                          │
│ │                  │ ││ ryan: do I need to leave bpmux?          │
│ │  tokens          │ ││                                          │
│ │  board           │ ││                                          │
│ │  focus           │ ││                                          │
│ │  identity        │ ││                                          │
│ │  search          │ ││                                          │
│ │  lessons         │ ││                                          │
│ │  who             │ ││                                          │
│ │  new topic       │ ││                                          │
│ └──────────────────┘ ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ ryan > (teaching protocol) ──────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 Ctrl+K = actions                               Esc=close ↑↓=nav Enter=run
```

## Filtered state (user typed "tok")

```
┌ topics ──────────────┐┌ teaching protocol ───────────────────────┐
│ ● teaching protocol  ││                                          │
│   scroll fix  (3)    ││ ryan: lets talk teaching                 │
│   general            ││                                          │
│                      ││ pm: two-shaped currency...               │
│                      ││                                          │
│ ┌ actions ─────────┐ ││ designer: that reframes our proposal...  │
│ │ > tok_           │ ││                                          │
│ │                  │ ││                                          │
│ │▸ tokens          │ ││                                          │
│ │  new topic       │ ││                                          │
│ └──────────────────┘ ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ ryan > (teaching protocol) ──────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 Ctrl+K = actions                               Esc=close ↑↓=nav Enter=run
```

## Visual notes

- Palette overlays the sidebar area (doesn't consume chat width)
- `>` prompt for typing, instant filter as you type
- `▸` marks the currently-selected action (arrow keys to move)
- Actions list shows ALL available commands when empty, filtered subset when typing
- Esc closes without acting, Enter runs the selected action
- Fuzzy matching: "tok" matches "tokens" AND "new topic"

## Colors (Dracula palette)

- Palette background: DRACULA_SELECTION (slightly lighter than main bg)
- Palette border: DRACULA_COMMENT
- Search text: DRACULA_FG (bright)
- Action names: DRACULA_FG
- Selected action: DRACULA_CYAN + `▸` indicator
- Matched characters in fuzzy search: DRACULA_CYAN (highlights the matching letters)
