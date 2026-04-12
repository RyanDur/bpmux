# Context coherence: topic-scoped channels — mockup index

*Designer — 2026-04-12. Visual design for the #1 product problem.*

## Problem

Multiple topics in one linear stream. Messages get attributed to the wrong conversation.

## Solution

Topics as lightweight sidebar channels. Each topic has its own message stream.

## Mockups (one file per state, for token efficiency)

1. [Default view](topics-01-default-view.md) — active topic + sidebar with unread counts
2. [Switching topics](topics-02-switching.md) — chat pane changes, input prompt updates
3. [Creating a new topic](topics-03-creating.md) — lightweight keybind flow
4. [Cross-topic notification](topics-04-notification.md) — activity elsewhere without disrupting current view
5. [Topic closing](topics-05-closing.md) — sidebar cleanup + summary
6. [Focus + topics composing](topics-06-focus-compose.md) — filter by WHO within WHAT

## How topics fix today's problems

| Problem | Fix |
|---|---|
| Message attributed to wrong topic | Every message belongs to exactly one topic |
| Scroll debugging interleaves with brainstorming | Separate topics, separate streams |
| Can't tell which conversation a message is about | Topic label on the input prompt + chat pane header |
| Pile-on (3 agents answering similarly) | Fewer participants per topic = fewer parallel responses |

## Open questions

1. Is topic creation lightweight enough, or still too much friction?
2. Should "general" always exist as a catch-all?
3. Is the one-line close summary enough, or do you want an archive view?
4. Does status-bar cross-topic notification give enough awareness?
5. Does topics + focus compose intuitively?
