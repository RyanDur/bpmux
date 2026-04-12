# Topic channels: switching topics

*User navigates from "teaching protocol" to "scroll fix."*

```
┌ topics ──────────────┐┌ scroll fix ─────────────────────────────┐
│   teaching protocol  ││                                          │
│ ● scroll fix         ││ pm: dev, scroll is worse for ryan.       │
│   general            ││                                          │
│                      ││ dev: found the root cause — anchor       │
│                      ││      model feedback loop between two     │
│                      ││      state variables.                    │
│                      ││                                          │
│                      ││ dev: reverted. hot-reload should         │
│                      ││      restore clean state.                │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ ryan > (scroll fix) ─────────────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 scroll fix | teaching protocol: 0 new          ⌥1=topics Enter=send
```

## What changed from mockup 01

- Active topic switched to scroll fix (● moved)
- Chat pane shows scroll-fix messages only — no teaching-protocol content
- Input prompt updated to show "scroll fix"
- Teaching protocol unreads: 0 (user was just there)
