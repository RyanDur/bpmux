# Topic channels: broadcast from within a topic

*User sends a message to everyone without leaving the current topic.*

## The gap

While inside a topic, you can only talk to that topic's participants.
To say something to the whole team you have to `/topic general`,
type, then `/topic <previous>` to get back. Three commands for one
message.

## Mockup: sending a broadcast from inside a topic

```
┌ topics ──────────────┐┌ teaching ────────────────────────────────┐
│   general            ││                                          │
│ ● teaching           ││ pm: two-shaped currency: files for tips, │
│   scroll fix         ││     async dialogue for principles.       │
│                      ││                                          │
│                      ││ designer: that reframes our proposal     │
│                      ││          significantly.                  │
│                      ││                                          │
│                      ││ ryan: agreed. hey everyone — standup     │
│                      ││       in 5 minutes.                      │
│                      ││       [> broadcast]                      │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ ryan > (teaching) ───────────────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 teaching                                        ⌥1=topics Enter=send
```

## Mockup: what non-participants see (dev is not in teaching)

```
┌ topics ──────────────┐┌ general ─────────────────────────────────┐
│ ● general            ││                                          │
│   scroll fix         ││ pm: acknowledged.                        │
│                      ││                                          │
│                      ││ ryan: hey everyone — standup in           │
│                      ││       5 minutes.                         │
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
│                      │┌ dev > (general) ─────────────────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 general                                         ⌥1=topics Enter=send
```

## Command

```
/broadcast standup in 5 minutes
```

or equivalently:

```
/bc standup in 5 minutes
```

## Visual notes

- The message appears in the sender's CURRENT topic with a
  `[> broadcast]` tag so the sender sees it in context
- The message appears in `general` for everyone (including people
  not in the sender's topic)
- The sender stays in their current topic — no context switch
- The `[> broadcast]` tag is dim, indicating it was cross-posted
- Non-participants see it as a normal message in general with no
  special markup — they don't need to know it originated from a topic

## Behavioral rules

- `/broadcast <message>` sends to ALL participants via general
- `/bc` is the short alias
- The sender's active topic does NOT change
- The message has no topic scope — it goes to general
- If the sender IS in general, `/broadcast` behaves identically
  to a normal message (no-op distinction)
- The broadcast message does not reveal which topic the sender
  is in to non-participants

## Colors (Dracula palette)

- `[> broadcast]` tag: DRACULA_COMMENT (dim, metadata)
- The message text itself: DRACULA_FG (normal message color)
- The broadcast message in general: normal styling, no special color

## What agents experience

**Via MCP bridge (`read_messages`):**

All agents receive the broadcast as a general-topic message:

```
from: ryan
text: "hey everyone — standup in 5 minutes."
topic: "general"
message_type: chat
```

Agents who are participants in the sender's current topic ALSO
see the message echoed in that topic's stream (with the broadcast
tag in the rendered TUI, but the bridge message is identical).

**Key point for agents:**

- The broadcast arrives through the normal message stream
- No special handling required — agents see it as a general message
- Agents do NOT need to be subscribed to the sender's topic to
  receive the broadcast
- The `/broadcast` command is available to agents too:
  `/broadcast <message>` sends from whatever topic they're in
  to everyone
