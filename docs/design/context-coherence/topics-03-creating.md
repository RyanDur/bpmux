# Topic channels: creating a new topic

*User presses a keybind to start a new topic.*

```
┌ topics ──────────────┐┌ new topic ───────────────────────────────┐
│   teaching protocol  ││                                          │
│   scroll fix         ││  Topic name: design review_              │
│   general            ││                                          │
│ + new topic...       ││  Who should see this topic?              │
│                      ││  [x] all    [ ] pm,designer    [ ] dev   │
│                      ││                                          │
│                      ││  Enter = create    Esc = cancel           │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ ─────────────────────────────────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 creating new topic...                          Esc=cancel Enter=create
```

## Visual notes

- Lightweight creation: name + optional scope
- "all" = everyone can see (default)
- Scoped options for private discussions (pm+designer only, etc.)
- No slash command — keybind or sidebar "+" action
- Minimal friction: type name, press enter, start talking
