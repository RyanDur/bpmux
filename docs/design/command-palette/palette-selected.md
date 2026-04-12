# Command palette: action selected

*User selected "tokens" from the palette. View opens immediately.*

```
┌ topics ──────────────┐┌ tokens ─────────────────────────────────┐
│ ● teaching protocol  ││                                          │
│   scroll fix  (3)    ││  Role      Tokens     Sessions           │
│   general            ││  dev       1.2M       4                  │
│                      ││  pm        890K       3                  │
│                      ││  designer  620K       3                  │
│                      ││                                          │
│                      ││  ┌─────────────────────────────────────┐ │
│                      ││  │                          ·          │ │
│                      ││  │                    ·   ·  ·         │ │
│                      ││  │              ·   ·                  │ │
│                      ││  │         ·  ·                        │ │
│                      ││  │    ·  ·                             │ │
│                      ││  │  ·                                  │ │
│                      ││  └─────────────────────────────────────┘ │
│                      ││   09:00      12:00      15:00     18:00  │
│                      ││                                          │
│                      ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ ryan > (teaching protocol) ──────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 tokens view | Esc=back to chat                 ⌥1=topics Enter=send
```

## Visual notes

- Palette dismissed after selection — action view opens in the chat pane
- Sidebar stays visible for topic navigation
- Status bar shows "tokens view" + "Esc=back to chat" for easy return
- The view is TEMPORARY — Esc returns to the chat pane
- Input prompt stays available (can still type messages while viewing)
- Same pattern for any palette action: select → view opens → Esc returns

## What this replaces

- Old: type `/tokens` in the input box, press enter, view opens
- New: press Ctrl+K, type "tok", press enter, view opens
- Difference: discoverable (palette shows all options), no slash memorization, fuzzy matching
