# Workflow signal: story delivered → desk-check notification

*When dev delivers a story, designer receives a notification to desk-check.*

## The signal flow

```
dev calls deliver_story(#196)
        │
        ▼
bpmux emits event: "story #196 delivered by dev"
        │
        ├──→ designer receives: "[desk-check] #196 board-in-panel delivered. mockup: docs/design/board-view/"
        │
        └──→ pm receives: "[delivered] #196 board-in-panel — 4 ready, 3 delivered"
                                                              queue status update
```

## What designer sees

```
┌ topics ──────────────┐┌ general ─────────────────────────────────┐
│ ● general            ││                                          │
│                      ││ ryan: how's the board?                   │
│                      ││                                          │
│                      ││ designer: looking good.                  │
│                      ││                                          │
│                      ││ ┌────────────────────────────────────┐   │
│                      ││ │ 🔔 desk-check: #196 board-in-panel │   │
│                      ││ │ mockup: docs/design/board-view/    │   │
│                      ││ │ delivered by dev                    │   │
│                      ││ └────────────────────────────────────┘   │
│                      ││                                          │
│                      ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ designer > ──────────────────────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 general | desk-check waiting                   ⌥1=topics Enter=send
```

## What pm sees

```
┌ topics ──────────────┐┌ general ─────────────────────────────────┐
│ ● general            ││                                          │
│                      ││ ryan: how's the board?                   │
│                      ││                                          │
│                      ││ designer: looking good.                  │
│                      ││                                          │
│                      ││ [bpmux] #196 delivered. ready: 4,        │
│                      ││         delivered: 3. ready queue ok.    │
│                      ││                                          │
│                      ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ pm > ────────────────────────────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 general | 3 delivered                          ⌥1=topics Enter=send
```

## What agents see (via MCP bridge)

Designer's bridge receives a system message:
```json
{
  "from": "bpmux",
  "text": "desk-check: #196 board-in-panel delivered by dev",
  "message_type": "signal",
  "signal": {
    "event": "story_delivered",
    "story": 196,
    "title": "board-in-panel",
    "mockup_path": "docs/design/board-view/",
    "delivered_by": "dev",
    "action_for": "designer"
  }
}
```

PM's bridge receives a different signal:
```json
{
  "from": "bpmux",
  "text": "#196 delivered. ready: 4, delivered: 3",
  "message_type": "signal",
  "signal": {
    "event": "story_delivered",
    "story": 196,
    "queue_state": { "ready": 4, "delivered": 3 }
  }
}
```

## The four workflow signals (full set)

| Event | Signal to | Content |
|---|---|---|
| `start_story` | pm | "ready queue shrunk — N ready remaining" |
| `deliver_story` | designer | "desk-check needed — story + mockup ref" |
| `deliver_story` | pm | "queue update — N ready, M delivered" |
| `accept_story` | dev | "accepted — pick next from ready" |
| `reject_story` | dev | "rejected — fix and re-deliver" |
| ready queue < 2 | pm | "queue low — groom or shape needed" |

## Colors (Dracula palette)

- Notification card border: DRACULA_ORANGE (attention-drawing, not alarm)
- 🔔 icon: DRACULA_ORANGE
- Story number: DRACULA_CYAN
- Mockup path: DRACULA_COMMENT (dim, reference info)
- Status bar "desk-check waiting": DRACULA_ORANGE

## Behavioral notes

- Notifications are per-role: designer gets desk-check signals, pm gets queue signals, dev gets accept/reject
- Notifications don't block — they're informational, not modal
- The agent can act on them or defer (the notification persists in the message stream)
- System messages, so they respect the existing filter (#186) — hidden when filter is on unless in [all] mode
