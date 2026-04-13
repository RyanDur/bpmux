# Topic channels: leaving a topic

*A participant leaves a topic they no longer need to follow.*

## The gap

Once you're in a topic, there's no way out except closing the
entire topic. You can switch away, but messages keep arriving as
unreads. There's no `/topic leave` to unsubscribe.

## Mockup: before leaving

```
┌ topics ──────────────┐┌ scroll fix ─────────────────────────────┐
│   general            ││                                          │
│   teaching           ││ dev: reverted. hot-reload should         │
│ ● scroll fix         ││      restore clean state.                │
│                      ││                                          │
│                      ││ pm: confirmed, scroll is clean now.      │
│                      ││                                          │
│                      ││ designer: looks good from the UX side.   │
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
│                      │┌ designer > (scroll fix) ─────────────────┐
│                      ││ /topic leave                             │
└──────────────────────┘└──────────────────────────────────────────┘
 scroll fix                                      ⌥1=topics Enter=send
```

## Mockup: after leaving

```
┌ topics ──────────────┐┌ general ─────────────────────────────────┐
│ ● general            ││                                          │
│   teaching           ││ dev: commit pushed, hot-reload should    │
│                      ││      pick it up.                         │
│                      ││                                          │
│                      ││ [you left topic: scroll fix]             │
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
│                      ││                                          │
│                      ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ designer > (general) ────────────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 general                                         ⌥1=topics Enter=send
```

## Visual notes

- `/topic leave` removes the topic from your sidebar
- View falls back to `general` (always exists, can't leave it)
- A one-line system note appears: `[you left topic: scroll fix]`
- The topic continues for other participants — leaving is personal
- No confirmation prompt. Leave is immediate and reversible
  (someone can re-invite you, or you can `/topic scroll fix` to
  rejoin if still open)
- Sidebar updates: "scroll fix" disappears

## Behavioral rules

- `/topic leave` operates on the ACTIVE topic
- You cannot leave `general` — it's the catch-all
- Leaving unsubscribes you: no more messages from that topic
- Other participants see a system note:
  `[designer left the topic]`
- The topic creator CAN leave their own topic
- If all participants leave, the topic auto-closes (same as
  topics-05 close behavior)
- Rejoining: `/topic <name>` re-subscribes you if the topic
  is still open

## Colors (Dracula palette)

- `[you left topic: scroll fix]`: DRACULA_COMMENT (dim, informational)
- `[designer left the topic]` (other participants' view): DRACULA_COMMENT
- Sidebar removal: topic simply disappears, no animation

## What agents experience

**Via MCP bridge (`read_messages`):**

When a participant leaves, remaining participants receive:

```
from: bpmux
text: "designer left topic 'scroll fix'."
topic: "scroll fix"
message_type: system
```

The leaving agent receives:

```
from: bpmux
text: "You left topic 'scroll fix'. Switched to general."
topic: null
message_type: system
```

**After leaving:**

- The agent stops receiving messages from that topic
- The topic no longer appears in `/topic` list output
- If the agent tries to send to the topic, they get an error:
  "You are not a participant in 'scroll fix'. Use /topic scroll fix
  to rejoin."

**Rejoining:**

- `/topic <name>` re-subscribes the agent if the topic still exists
- The agent then receives new messages going forward (not backfill)
