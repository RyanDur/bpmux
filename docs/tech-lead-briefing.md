# Tech Lead Briefing — April 4, 2026

While you were offline, the PM and owner completed a full research and planning session. Here's everything you need to know, plus questions for you.

## What changed in the repo

- **ROADMAP.md** — three-horizon roadmap mapping all 28 open issues. Read this first.
- **AGENT_WORKFLOW.md** — Designer added as 4th role. Quality review now includes design/UX bot.
- **README.md** — full setup guide for new users including bridge, plugin, keybindings.
- **docs/research/** — 93 files reorganized into 8 topic directories. 13 new research docs added.
- **.mcp.json** — untracked from git (machine-specific paths). Added to .gitignore.
- **Commit a690504** has all the research/workflow changes. **Commit f9b8d26** has the roadmap.

## New stories filed

**Ready (priority order — pull from top):**
1. **#18** Truncate long AI messages (render-layer, ~5 lines of code)
2. **#19** Per-agent colors (render-layer, ~10 lines)
3. **#20** Terminal bell on new messages (1 line: emit \x07)
4. **#21** Unread message indicator when scrolled up (~20 lines)
5. **#22** @-mention addressing (parse @name, highlight in render)
6. **#23** Agent activity indicator in status bar (new message_type: "activity")
7. **#16** Auto-scroll fix
8. **#15** Input box visibility
9. **#14** TUI border rendering
10. **#17** Esc safety (change Esc to clear input, /quit or Ctrl-C to exit)

**Also filed (#24-#31):** Input history, onboarding, help overlay, /who, join/leave events, read cursor, reconnection backfill, debug toggle.

**Icebox (need your input):**
- **#32** `bpmux connect <tool>` command
- **#33** MLX model as bpmux participant
- **#34** Identity protocol (`name:tool` instead of `:ws`)

## Key research findings that affect your work

### 1. Conduit model evolved: "opinionated conduit"
The conduit model holds but needs to be more expressive. Like SMTP: structure opinions yes, behavior opinions no. Three additions proposed:
- **Message IDs** — add `id: String` to Message struct. Invisible, zero-risk, unlocks everything else (reply references, read cursors, ack targeting). The designer says this is the single most important structural change.
- **`in_reply_to: Option<String>`** — cheap to add now, expensive to retrofit later. Pull as a chore before @-mention addressing (#22).
- **Expanded `message_type`** — already exists as `Option<String>`. Consider "activity" type for #23.

Read: `docs/research/architecture/conduit-model-reevaluation.md`

### 2. The TUI viewport problem is an AI problem
Session data analysis (521 messages across 3 sessions): median human message is 54 chars. Messages over 390 chars (5 wrapped lines) are almost exclusively from AI. One long AI message fills the entire 80x24 viewport. That's why #18 (truncation) is the #1 priority.

Read: `docs/research/process/session-analysis.md`

### 3. 7/10 coding agents can connect TODAY
bpmux's Streamable HTTP MCP endpoint works with Copilot, Cursor, Windsurf, Cline, Continue, Gemini, and Amazon Q — zero code changes needed. Only Aider lacks MCP. Zed and OpenCode need a thin bridge.

Read: `docs/research/architecture/cross-tool-integration.md`

### 4. Clark & Brennan: AI agents skip the acceptance phase
Communication isn't just transmission — it's joint establishment of mutual belief. bpmux handles presentation (sending messages) but provides no structural support for the acceptance phase (evidence of understanding). AI agents won't seek or provide grounding evidence on their own. The message ID and in_reply_to fields are the minimum structural investment to enable grounding.

Read: `docs/research/foundations/clark-brennan-deep-read.md`

### 5. Edmondson: first interactions determine trajectory
Psychological safety is the single strongest predictor of learning behavior. The first few interactions with bpmux determine whether it feels safe or like surveillance. This means onboarding (#25) and /help (#26) aren't polish — they're load-bearing.

Read: `docs/research/foundations/edmondson-deep-read.md`

### 6. Channels dependency is HIGH risk
`--dangerously-load-development-channels` is experimental. No stability guarantee. Anthropic can break it without notice (precedent: Claude 4.6 removed prefilling with a hard 400 error). Our fallback already works: hooks provide per-turn sync. WebRTC (#5) is the long-term path that doesn't depend on experimental APIs.

### 7. TUI mockups exist
ASCII mockups for #18-#21 are at `docs/research/process/tui-mockups.md`. These are reference designs — feel free to deviate.

## Design principles (10 research-backed rules)
Read: `docs/research/process/design-principles.md`

The ones most relevant to your immediate work:
- **Principle 1:** Messages must be readable without re-reading (truncation)
- **Principle 2:** Participants need who/what/where awareness (colors, activity)
- **Principle 7:** Respect terminal conventions (readline, vim scroll)
- **Principle 8:** Make quitting safe and destruction recoverable (Esc → clear input)

## Questions for you

1. **Message IDs:** What format? UUID, monotonic sequence, or timestamp-based? The designer recommends adding the field now even if nothing uses it yet.

2. **in_reply_to:** Should this go in before @-mentions (#22), or is it premature? The PM says cheap structural investment. What do you think?

3. **Identity protocol (#34):** The WebSocket handler hardcodes `:ws`. The proposal is `{"type": "identify", "name": "pm", "tool": "claude-code"}` → identity `pm:claude-code`. Does this break anything?

4. **Truncation (#18):** The designer says "clamp to 3 visible lines + [+N lines]". Is there a ratatui-native way to do this, or is it a manual line-count in the render function?

5. **Activity indicator (#23):** This needs a new `message_type: "activity"` convention. Agents broadcast `/working-on <description>` as an activity message. The status bar shows the most recent activity per peer. Feasible?

6. **Anything we missed?** You've been building this codebase. What do you see that the research and design team doesn't?

## The thesis (updated)

> bpmux is collective memory for human-AI teams. The persistent log is the primitive. Everything else is built on top of it. Communication is not transmission — it is the joint establishment of mutual belief.

Read: `docs/research/synthesis/thesis-v2.md`
