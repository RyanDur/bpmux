# Topic channels: default view

*One active topic, others visible in sidebar with unread counts.*

```
┌ topics ──────────────┐┌ teaching protocol ───────────────────────┐
│ ● teaching protocol  ││                                          │
│   scroll fix  (3)    ││ ryan: lets talk teaching                 │
│   general            ││                                          │
│                      ││ pm: two-shaped currency: files for tips, │
│                      ││     async dialogue for principles.       │
│                      ││                                          │
│                      ││ designer: that reframes our proposal     │
│                      ││           significantly.                 │
│                      ││                                          │
│                      ││ ryan: do I need to leave bpmux?          │
│                      ││                                          │
│                      ││ pm: no. everything inside bpmux.         │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ ryan > (teaching protocol) ──────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 teaching protocol | scroll fix: 3 new          ⌥1=topics Enter=send
```

## Visual notes

- Sidebar lists TOPICS, not roles
- `●` marks the active topic
- `(3)` shows unread count — dev is active there
- Input prompt shows which topic you're typing into
- Status bar: active topic + other-topic activity at a glance
- `general` is the catch-all for non-topic messages

## Colors (Dracula palette)

- Active topic name: DRACULA_CYAN, bold
- Inactive topic with unreads: DRACULA_ORANGE + count
- Inactive topic, no unreads: DRACULA_COMMENT (dim)
- Input prompt topic label: DRACULA_PURPLE
- Status bar topic activity: DRACULA_ORANGE for unreads
