# Topic channels: cross-topic notification

*A message arrives in another topic while the user is reading the current one.*

```
┌ topics ──────────────┐┌ teaching protocol ───────────────────────┐
│ ● teaching protocol  ││                                          │
│   scroll fix  (1)    ││ pm: the consulting model reshapes the    │
│   design review      ││     protocol entirely.                   │
│   general            ││                                          │
│                      ││ ryan: that sounds right. you may need    │
│                      ││       to adjust based on the type of     │
│                      ││       communication you receive.         │
│                      ││                                          │
│                      ││ designer: that adds an important         │
│                      ││          dimension — the protocol        │
│                      ││          can't be rigid.                 │
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
 teaching protocol | scroll fix: 1 new (dev)    ⌥1=topics Enter=send
```

## Visual notes

- New message arrived in "scroll fix" from dev
- Sidebar shows `(1)` unread on scroll-fix
- Status bar: "scroll fix: 1 new (dev)" — who posted, without leaving current topic
- Viewport NOT disrupted — user keeps reading teaching protocol
- User glances at status bar and decides: switch or keep reading
