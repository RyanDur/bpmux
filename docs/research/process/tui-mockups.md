# TUI Mockups — Top 4 Stories

Reference designs for 80x24 terminal. Based on current layout in `src/terminal/ui.rs`:
- Chat pane: flexible height, bordered, title " bpmux "
- Input pane: 3 rows (1 content + 2 border), bordered, title " > "
- Status bar: 1 row, no border

Current color scheme: human names = cyan, AI names = green.
Current status bar: `{connection} | {session_id} [| scrolled up N]`

---

## Mockup 1: #18 -- Message Truncation

Long AI messages are collapsed to 3 lines with a clickable/visible `[+N lines]` hint.
Short messages render normally. This keeps the chat scannable.

```
+-- bpmux ----------------------------------------------------------------+
|                                                                          |
| ryan: what's the status on the websocket refactor?                       |
|                                                                          |
| dev: I've finished the WebSocket refactor. Here's what changed:          |
|   1. Moved connection pooling to transport layer                         |
|   2. Added heartbeat pings every 30s                                     |
|   [+14 lines]                                                            |
|                                                                          |
| ryan: nice, let's ship it                                                |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
+--------------------------------------------------------------------------+
+-- > ---------------------------------------------------------------------+
|                                                                          |
+--------------------------------------------------------------------------+
 2 connected | abc123-session
```

Design notes:
- Default collapsed height: 3 lines of content, then `[+N lines]`
- The `[+N lines]` indicator is styled dim/gray so it doesn't compete with content
- Full message is still in the log -- this is display-only truncation
- Future: pressing Enter on a truncated message could expand it

---

## Mockup 2: #19 -- Per-Agent Colors

Each unique sender gets a distinct color. Humans and AIs are distinguishable
at a glance. Color annotations shown in brackets for this ASCII mockup.

```
+-- bpmux ----------------------------------------------------------------+
|                                                                          |
| [cyan] ryan: kicked off the session, let's align on priorities           |
|                                                                          |
| [green] dev: I've been looking at the transport layer. The WebSocket     |
| handshake needs work -- we're dropping connections on slow networks.     |
|                                                                          |
| [yellow] pm: Sounds good. Can we get an estimate on that? I want to     |
| make sure it fits in this iteration.                                     |
|                                                                          |
| [cyan] ryan: should be a day or two, it's mostly plumbing               |
|                                                                          |
| [green] dev: Agreed. I'll have a PR up by EOD tomorrow.                  |
|                                                                          |
| [yellow] pm: Perfect. I'll update the board.                             |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
+--------------------------------------------------------------------------+
+-- > ---------------------------------------------------------------------+
|                                                                          |
+--------------------------------------------------------------------------+
 3 connected | sprint-planning-42
```

Design notes:
- Color assignment: deterministic hash of sender name to color palette
- Palette: cyan, green, yellow, magenta, blue, red (6 colors, wraps)
- Human vs AI distinction preserved: humans get bold names, AIs get normal weight
- Same sender always gets the same color across sessions (hash-based)

---

## Mockup 3: #21 -- Unread Indicator

When the user scrolls up and new messages arrive, the status bar shows
an unread count. When at the bottom, no indicator appears.

### State A: Scrolled up, 3 new messages arrived

```
+-- bpmux ----------------------------------------------------------------+
|                                                                          |
| ryan: here's the initial design                                          |
|                                                                          |
| dev: looks good, a few questions:                                        |
| - should we handle reconnection in the bridge or transport?              |
| - what's the timeout for stale connections?                              |
|                                                                          |
| ryan: transport layer owns reconnection, 60s timeout                     |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
+--------------------------------------------------------------------------+
+-- > ---------------------------------------------------------------------+
|                                                                          |
+--------------------------------------------------------------------------+
 2 connected | design-session | scrolled up 12 | 3 new messages
```

### State B: At bottom (no indicator)

```
+-- bpmux ----------------------------------------------------------------+
|                                                                          |
| ryan: transport layer owns reconnection, 60s timeout                     |
|                                                                          |
| dev: makes sense. I'll draft the reconnection logic.                     |
|                                                                          |
| pm: quick note -- client demo is Thursday, let's make sure               |
| reconnection is stable by then.                                          |
|                                                                          |
| ryan: we'll have it. dev, ping me when the PR is up.                     |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
+--------------------------------------------------------------------------+
+-- > ---------------------------------------------------------------------+
|                                                                          |
+--------------------------------------------------------------------------+
 2 connected | design-session
```

Design notes:
- Unread count resets to 0 when scroll_offset returns to 0 (user at bottom)
- `3 new messages` is styled with a highlight color (e.g., bright yellow)
- Count increments via `receive_message` when `scroll_offset > 0`
- Story #20 (terminal bell) fires at the same time: bell on new message when scrolled up

---

## Mockup 4: Combined -- Target State

All 4 improvements together: truncation (#18), per-agent colors (#19),
terminal bell (#20), and unread indicator (#21).

### Active conversation, scrolled up, new messages waiting

```
+-- bpmux ----------------------------------------------------------------+
|                                                                          |
| [cyan] ryan: what's everyone working on today?                           |
|                                                                          |
| [green] dev: I'm finishing the WebSocket heartbeat. Here's the plan:     |
|   - Add ping/pong frames every 30s                                       |
|   - Detect dead connections after 3 missed pongs                         |
|   [+8 lines]                                                             |
|                                                                          |
| [yellow] pm: I'm updating the backlog. We have 12 stories in the        |
|   icebox that need grooming. I've drafted acceptance criteria for the    |
|   [+5 lines]                                                             |
|                                                                          |
| [cyan] ryan: sounds good. dev, can you review my transport PR            |
|   before you start on heartbeat?                                         |
|                                                                          |
| [green] dev: sure, I'll look at it first thing                           |
|                                                                          |
|                                                                          |
+--------------------------------------------------------------------------+
+-- > ---------------------------------------------------------------------+
|                                                                          |
+--------------------------------------------------------------------------+
 3 connected | sprint-day-5 | scrolled up 8 | 2 new messages  [BEL]
```

### Same conversation, at bottom, all caught up

```
+-- bpmux ----------------------------------------------------------------+
|                                                                          |
| [green] dev: sure, I'll look at it first thing                           |
|                                                                          |
| [yellow] pm: great. also, the client wants a demo of the multi-agent    |
|   chat on Thursday. Can we have something stable by then?                |
|                                                                          |
| [cyan] ryan: yes. truncation and colors should be done by tomorrow.      |
|   unread indicator is small. we'll be ready.                             |
|                                                                          |
| [green] dev: I'll make sure the bridge handles reconnection              |
|   gracefully so we don't drop messages during the demo.                  |
|                                                                          |
| [yellow] pm: perfect. I'll send the invite.                              |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
|                                                                          |
+--------------------------------------------------------------------------+
+-- > ---------------------------------------------------------------------+
|                                                                          |
+--------------------------------------------------------------------------+
 3 connected | sprint-day-5
```

Design notes:
- `[BEL]` annotation represents the terminal bell (story #20); in reality it is
  an invisible `\x07` character written to stdout, not displayed text. The TUI
  itself shows nothing -- the user's terminal emits the audible/visual bell.
- Bell fires once per batch of new messages, not per message (avoids spam).
- Truncation + colors + unread indicator work independently. No coupling between
  the three features. Each story can ship and be tested in isolation.
- The combined effect: you can scan a busy multi-agent chat quickly (colors),
  long AI output doesn't dominate the viewport (truncation), you know when
  you've missed something (unread count + bell), and you can scroll back to
  catch up.
