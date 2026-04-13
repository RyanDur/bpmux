# Topic channels: cross-topic activity notification

*While focused on one topic, the user sees activity in other topics
without leaving.*

## The gap

Topics-04 showed the basic status-bar notification (one line). In
practice during live testing, that wasn't enough. When three topics
are active simultaneously, a single status-bar line can't convey
which topics have activity, from whom, or how much. You end up
switching topics to check, which defeats the purpose.

## Mockup: multiple topics active, activity in two of them

```
┌ topics ──────────────┐┌ teaching ────────────────────────────────┐
│   general            ││                                          │
│ ● teaching           ││ pm: two-shaped currency: files for tips, │
│   scroll fix  (3)    ││     async dialogue for principles.       │
│   design rev  (1)    ││                                          │
│                      ││ designer: that reframes our proposal.    │
│                      ││                                          │
│                      ││ ryan: do I need to leave bpmux?          │
│                      ││                                          │
│                      ││ pm: no. everything inside bpmux.         │
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
 teaching | scroll fix: 3 new | design rev: 1 new ⌥1=topics Enter=send
```

## Mockup: sidebar peek — hover/select a topic to see last message

```
┌ topics ──────────────┐┌ teaching ────────────────────────────────┐
│   general            ││                                          │
│ ● teaching           ││ pm: two-shaped currency: files for tips, │
│ ▸ scroll fix  (3)    ││     async dialogue for principles.       │
│   design rev  (1)    ││                                          │
│                      ││ designer: that reframes our proposal.    │
│┌─────────────────────┐│                                          │
││ dev: found the root ││ ryan: do I need to leave bpmux?          │
││ cause — anchor mode ││                                          │
││ feedback loop.      ││ pm: no. everything inside bpmux.         │
│└─────────────────────┘│                                          │
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
 teaching | scroll fix: 3 new | design rev: 1 new ⌥1=topics Enter=send
```

## Visual notes

- **Status bar** lists ALL topics with unreads, not just one.
  Format: `topic: N new` separated by `|`
- **Sidebar unread counts** show `(N)` next to each topic with
  activity (same as topics-01)
- **Sidebar peek**: arrow-key to a topic with unreads and a
  tooltip shows the last message preview without switching.
  The `▸` marker shows which sidebar item is selected.
- The peek tooltip overlays the sidebar area, not the chat pane.
  Chat content is NOT disrupted.
- Enter on the selected sidebar item switches to that topic.
  Esc or arrow-away dismisses the peek.

## Behavioral rules

- Status bar shows up to 3 topics with unreads. If more than 3,
  show `+N more` at the end
- Sidebar peek shows the LAST message only (not a scrollable
  preview — that's what switching is for)
- Unread counts reset when you switch to a topic (existing
  behavior from topics-01)
- No sound, no flash, no modal. Notification is ambient: sidebar
  counts + status bar text
- The peek is keyboard-navigable: arrow keys move selection in
  the sidebar, Enter switches, Esc dismisses

## Colors (Dracula palette)

- Sidebar unread count `(3)`: DRACULA_ORANGE
- Sidebar selected item `▸`: DRACULA_CYAN
- Peek tooltip border: DRACULA_SELECTION
- Peek tooltip text: DRACULA_FG
- Status bar topic activity: DRACULA_ORANGE for the counts
- Status bar separator `|`: DRACULA_COMMENT

## What agents experience

**Via MCP bridge (`read_messages`):**

Agents already receive all messages they're subscribed to,
regardless of which topic is "active." Cross-topic notification
is a TUI-only problem — agents don't have a viewport to be
disrupted.

However, agents benefit from one behavioral detail:

When an agent calls `read_messages`, messages include the `topic`
field. The agent can filter or prioritize by topic:

```
from: dev
text: "found the root cause — anchor model feedback loop."
topic: "scroll fix"
```

```
from: pm
text: "acknowledged."
topic: "design rev"
```

**What this means for agents:**

- Agents don't need cross-topic notification — they see all
  subscribed messages in one stream
- The `topic` field lets agents decide what to respond to
- Agents should check the topic field before responding to avoid
  posting in the wrong context
- An agent in topic A that wants to respond to activity in topic B
  should `/topic B` first, then respond
