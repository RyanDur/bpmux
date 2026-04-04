# bpmux Session Log Analysis

Quantitative analysis of 3 dogfooding sessions from `~/.bpmux/sessions/`.

**Date of analysis:** 2026-04-04
**Data source:** JSONL message logs from bpmux session persistence

---

## Summary

| Metric | Value |
|--------|-------|
| Sessions | 3 |
| Total messages | 521 |
| Substantive messages (after filtering) | 336 (64.5%) |
| "[turn complete]" noise | 174 (33.4%) |
| System messages | 38 (7.3%) |
| Unique senders | 6 identities across 3 sessions |

**The single most important finding:** one-third of all stored messages are `[turn complete]` markers from the Claude Code hook. These are protocol noise, not conversation. Any design work based on raw message counts would overcount by ~50%.

---

## Per-Session Breakdown

### Session 1: ce31f32b (the long dogfood)

| Metric | Value |
|--------|-------|
| Duration | 30h 10m (Apr 2 22:24 - Apr 4 04:35 UTC) |
| Messages | 414 |
| Participants | 6 identities |
| Median gap | 13.3s |
| Gaps >60s | 87 |
| Longest gap | 1016 min (~17 hours, overnight) |

**Participants:**
- `ryan:claude-code-user` (144 msgs, 34.8%) -- user-side hook capturing what Ryan typed into Claude Code
- `ryan:claude-code` (139 msgs, 33.6%) -- Claude Code response hook; **all 139 substantive messages are `[turn complete]`**
- `ryan:mcp-client` (63 msgs, 15.2%) -- Claude Code agent calling send_message via MCP bridge
- `ryan:human` (38 msgs, 9.2%) -- Ryan typing directly in bpmux TUI
- `claude-code:ws` (28 msgs, 6.8%) -- Claude Desktop connected via WebSocket
- `keystroke_test:human` (2 msgs, 0.5%) -- test identity

This session spans the full Apr 2-4 development period. It includes the WebSocket bridge buildout, Desktop integration testing, and story completion work. The 17-hour gap is overnight sleep.

### Session 2: 3b11c24a (hook debugging)

| Metric | Value |
|--------|-------|
| Duration | 34 min |
| Messages | 63 |
| Participants | 4 identities |
| System messages | 35 (55.6%) |
| Median gap | 21.4s |

Majority system-type messages. This session was primarily testing the hook mechanism and system message filtering. Short, focused debugging session.

### Session 3: 3c738242 (active pairing)

| Metric | Value |
|--------|-------|
| Duration | 18 min |
| Messages | 44 |
| Participants | 3 identities |
| Median gap | 9.7s |
| System messages | 0 |

The cleanest session. `claude-code:ws` (25 msgs, 56.8%) and `ryan:human` (14 msgs, 31.8%) in active conversation. This is the closest to the intended use case: human and AI agent exchanging substantive messages through bpmux.

---

## Sender Identity Analysis

Six distinct `from` identities appear across sessions. They map to three actual roles:

### Human (Ryan)
- **`ryan:human`** -- typed directly in bpmux TUI (59 msgs total)
- **`ryan:claude-code-user`** -- captured by Claude Code send-user-message hook (156 msgs total)

### AI Agent (Claude)
- **`ryan:claude-code`** -- captured by Claude Code send-claude-response hook (174 msgs; **100% are `[turn complete]` after filtering**)
- **`ryan:mcp-client`** -- Claude Code calling send_message via MCP bridge (77 msgs)
- **`claude-code:ws`** -- Claude Desktop connected via WebSocket (53 msgs)

### Test
- **`keystroke_test:human`** -- test identity (2 msgs)

**Key observation:** Claude's actual substantive messages come through `ryan:mcp-client` and `claude-code:ws`. The `ryan:claude-code` identity is purely hook protocol overhead.

---

## Message Content Analysis

### After filtering noise (336 substantive messages)

| Metric | Value |
|--------|-------|
| Mean length | 284 chars |
| Median length | 54 chars |
| Messages >390 chars (5+ wrapped lines) | 41 (12.2%) |

### Length distribution (all 521 messages)

| Bucket | Count | % |
|--------|-------|---|
| 0-50 chars | 340 | 65.3% |
| 51-200 chars | 88 | 16.9% |
| 201-390 chars | 42 | 8.1% |
| 391-1000 chars | 30 | 5.8% |
| 1001-5000 chars | 18 | 3.5% |
| 5000+ chars | 3 | 0.6% |

**The median message is 24 characters.** Two-thirds of all messages are under 50 chars. This is a short-message medium. The long tail (5000+ chars) comes from TUI frame captures sent through hooks -- 9 messages contain box-drawing characters, indicating raw TUI screenshots were captured as messages.

### Substantive message lengths tell a different story

Once `[turn complete]` markers are removed, the median jumps to 54 chars and the mean to 284 chars. AI agent messages are substantially longer than human messages. The distribution is bimodal: short human messages (commands, acknowledgments) and longer AI messages (explanations, status updates).

---

## Turn Patterns

### Session 1 (414 messages)
- Turn switches: 275
- Burst sequences (same sender >1 consecutive): 97
- Max burst: 7 messages from same sender
- Mean run length: 1.5

### Session 3 (44 messages, cleanest)
- Turn switches: 32
- Burst sequences: 11
- Max burst: 2 messages
- Mean run length: 1.3

**Pattern:** Mostly alternating, with occasional short bursts. The max burst of 7 in session 1 comes from `[turn complete]` markers firing in sequence. In the clean session 3, no sender ever sends more than 2 consecutive messages. This is genuine turn-taking conversation.

---

## Timing Patterns

| Session | Median gap | Mean gap | Max gap |
|---------|-----------|----------|---------|
| ce31f32b (30h) | 13.3s | 263.0s | 1016 min |
| 3b11c24a (34m) | 21.4s | 33.6s | 2.8 min |
| 3c738242 (18m) | 9.7s | 25.3s | 3.5 min |

**Active conversation pace is 10-20 seconds between messages.** The high mean in session 1 is skewed by overnight and break gaps (87 gaps >60s). During active pairing, the rhythm is fast -- closer to chat than email.

---

## Noise Budget

| Category | Count | % of total |
|----------|-------|-----------|
| `[turn complete]` markers | 174 | 33.4% |
| System messages | 38 | 7.3% |
| TUI frame captures | 9 | 1.7% |
| **Total noise** | **~221** | **~42%** |
| **Substantive messages** | **~300** | **~58%** |

Nearly half of all persisted messages are not human-readable conversation. This has implications for:
- **Scroll distance** -- users must scroll past noise to find real messages
- **Storage** -- TUI frame captures are 1900-2300 chars of box-drawing characters
- **Read API** -- consumers calling `GET /api/messages` or `read_messages` get noise mixed with signal

---

## Topic Indicators

Top words in substantive messages (excluding stopwords, min 4 chars):

| Word | Count | Context |
|------|-------|---------|
| complete | 188 | "[turn complete]" markers |
| turn | 186 | "[turn complete]" markers |
| bridge | 57 | WebSocket bridge development |
| bpmux | 47 | Self-referential (building bpmux in bpmux) |
| hello | 44 | Testing/greeting |
| test | 43 | Testing messages |
| messages | 40 | Message system development |
| function | 38 | Code discussion |
| story | 29 | Story-based workflow |
| scroll | 29 | TUI scroll feature work |
| working | 27 | Status updates |
| session | 26 | Session management |
| changes | 26 | Code changes |
| claude | 24 | Agent identity |

The vocabulary confirms this is a tool-building dogfood: the team is talking about bpmux while using bpmux. High frequency of "test", "hello", and "working" suggests significant time spent on connectivity testing rather than sustained design conversation.

---

## How We Actually Use bpmux

Based on this data:

1. **It is a fast-paced short-message medium.** Median 54 chars substantive, 10-20s between messages. More like Slack than email.

2. **AI agents dominate message volume.** Even after filtering `[turn complete]`, agents send more and longer messages than humans. The human role is directing and acknowledging; the AI role is reporting and explaining.

3. **42% of stored messages are noise.** `[turn complete]` markers, system messages, and TUI frame captures. The hook design captures everything, but most of it is not conversation.

4. **Sessions are bursty with long tails.** Active pairing bursts of 18-34 minutes, separated by long gaps (hours, overnight). The 30-hour session is really ~6-8 active windows within a long-running session.

5. **Multiple identities per person.** Ryan appears as 3 different senders (`ryan:human`, `ryan:claude-code-user`, `ryan:mcp-client`). Claude appears as 3 (`ryan:claude-code`, `ryan:mcp-client`, `claude-code:ws`). This fragments the conversation in the log.

6. **The cleanest session is the most recent one.** Session 3 (18 min, 44 msgs, 0 system messages) shows the product converging toward its intended use: two participants exchanging substantive messages with clean turn-taking.

7. **Most long messages are from AI agents.** Of the 41 messages over 390 chars, the vast majority come from `claude-code:ws` or `ryan:mcp-client`. Human messages rarely exceed 200 chars.

8. **Testing dominates content.** "hello", "test", "hi", "checking" -- the dogfood data is heavily weighted toward connectivity verification rather than sustained collaborative work. Real usage patterns may differ.

---

## Implications for Design

- **Filter before display.** `[turn complete]` markers should never reach the TUI chat pane. System messages need a separate treatment or suppression.
- **Collapse identities.** `ryan:human` and `ryan:claude-code-user` are the same person. The display should unify them.
- **Optimize for short messages.** The median message fits on one line. Multi-line message rendering is the exception, not the rule.
- **AI messages need more space.** When they do appear, they are 3-5x longer than human messages. The layout should accommodate asymmetric message lengths.
- **Session continuity matters.** A single session spans days with gaps. The TUI needs to handle time gaps gracefully (date separators, "17 hours later" markers).
- **The noise problem is solvable.** The 42% noise is concentrated in well-defined categories. Filtering `[turn complete]` alone recovers a third of the message budget.
