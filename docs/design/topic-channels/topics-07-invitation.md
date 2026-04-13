# Topic channels: invitation and auto-subscribe

*Ryan creates a topic with participants. Participants are notified and auto-subscribed.*

## The gap

When `/topic create teaching pm,designer` runs, pm and designer have
no idea the topic exists. They don't receive messages sent to it.
The creator talks into a void until participants manually switch.

## Mockup: creator's view after creating

```
┌ topics ──────────────┐┌ teaching ────────────────────────────────┐
│   general            ││                                          │
│ ● teaching           ││ [topic created — participants: ryan,     │
│                      ││  pm, designer. all notified.]            │
│                      ││                                          │
│                      ││ ryan: lets work through the teaching     │
│                      ││       protocol.                          │
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
│                      │┌ ryan > (teaching) ───────────────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 teaching                                        ⌥1=topics Enter=send
```

## Mockup: participant's view (pm) — invitation arrives

```
┌ topics ──────────────┐┌ general ─────────────────────────────────┐
│ ● general            ││                                          │
│   teaching  (new)    ││ dev: commit pushed, hot-reload should    │
│                      ││      pick it up.                         │
│                      ││                                          │
│                      ││ pm: acknowledged.                        │
│                      ││                                          │
│                      ││ ┌─ invitation ───────────────────────┐   │
│                      ││ │ ryan invited you to topic:         │   │
│                      ││ │ teaching  [pm, designer, ryan]     │   │
│                      ││ │                                    │   │
│                      ││ │ You're subscribed. Switch with     │   │
│                      ││ │ /topic teaching or sidebar click.  │   │
│                      ││ └────────────────────────────────────┘   │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      ││                                          │
│                      │└──────────────────────────────────────────┘
│                      │┌ pm > (general) ──────────────────────────┐
│                      ││                                          │
└──────────────────────┘└──────────────────────────────────────────┘
 general | teaching: new topic (ryan)            ⌥1=topics Enter=send
```

## Visual notes

- Invitation appears as a bordered card in the participant's current chat
- The topic appears in the sidebar immediately with `(new)` label
- Participant is AUTO-SUBSCRIBED: messages to `teaching` will reach
  them without any action
- The card says "you're subscribed" — no accept/decline ceremony
- Status bar shows the new topic activity so it's visible at a glance
- Sidebar click or `/topic teaching` switches to it when ready

## Behavioral rules

- Creator always auto-subscribes (already works)
- Named participants auto-subscribe at topic creation time
- Auto-subscribe means: messages sent to the topic arrive in the
  participant's message stream, and the topic appears in their sidebar
- No "accept invitation" step. Participation is immediate.
- `/topic leave` (see topics-08) is how you opt out after the fact

## Colors (Dracula palette)

- Invitation card border: DRACULA_ORANGE (attention, notification)
- Invitation text: DRACULA_FG
- Topic name inside card: DRACULA_CYAN, bold
- `(new)` sidebar label: DRACULA_ORANGE
- Status bar topic notification: DRACULA_ORANGE

## What agents experience

**Via MCP bridge (`read_messages`):**

When a topic is created with an agent as participant, the agent
receives a system message:

```
from: bpmux
text: "Topic 'teaching' created by ryan. You are a participant.
       Switch with /topic teaching"
topic: null (system-level, visible regardless of active topic)
to: ["ryan", "pm", "designer"]
```

The agent does NOT need to take action to receive future messages
in the topic. Auto-subscribe means the bridge delivers topic
messages to the agent's `read_messages` stream automatically.

**What the agent should do:**

- Note the topic exists and who the participants are
- Switch to it with `/topic teaching` when ready to engage
- Or stay in current topic — messages accumulate as unreads

**What the agent should NOT need to do:**

- No "accept" action required
- No manual subscription step
- No polling to discover new topics
