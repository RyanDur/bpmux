# Usage statistics: feature usage in the tokens view

*Extends the existing /tokens view with a feature-usage table.*

```
┌ tokens + usage ──────────────────────────────────────────────────────────────┐
│                                                                              │
│  Role      Tokens     Sessions            Feature         Uses   Last used   │
│  dev       1.2M       4                   /board           8     2 min ago   │
│  pm        890K       3                   /topic           5     12 min ago  │
│  designer  620K       3                   /focus           3     45 min ago  │
│                                           /tokens          2     now         │
│  ┌─────────────────────────────────────┐  /identity        1     3h ago      │
│  │                          ·          │  /help            0     —           │
│  │                    ·   ·  ·         │                                     │
│  │              ·   ·                  │                                     │
│  │         ·  ·                        │                                     │
│  │    ·  ·                             │                                     │
│  │  ·                                  │                                     │
│  └─────────────────────────────────────┘                                     │
│   09:00      12:00      15:00     18:00                                      │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
┌ ryan > ──────────────────────────────────────────────────────────────────────┐
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
 tokens + usage | Esc=back to chat                 ⌥1=topics Ctrl+K=actions
```

## Visual notes

- Extends the existing /tokens view — same surface, additional column
- Left side: token cost data (role breakdown + chart, already shipped in #179)
- Right side: feature usage table (new)
- Sorted by usage count descending — most-used features at top
- "Last used" gives recency context — a feature used 8 times 3 hours ago vs 8 times 2 min ago tells different stories
- Features with 0 uses are visible — they're the signal for "consider removing"

## Colors (Dracula palette)

- Feature names: DRACULA_FG
- Usage counts > 0: DRACULA_GREEN
- Usage count = 0: DRACULA_COMMENT (dim — unused features fade visually)
- "Last used" timestamps: DRACULA_COMMENT

## What this measures

- Every slash command invocation
- Every command-palette action selection
- Every sidebar navigation action
- NOT message sends (those are communication, not feature usage)
- NOT passive renders (the board opening isn't "using" individual stories)
