# Tech Lead Briefing — April 4, 2026

While you were offline, the PM and owner completed a full research and planning session. Here's everything you need to know, plus questions for you.

## How we work together now

**The balanced team is now four roles: Owner, PM, Designer, Tech Lead.** The designer reviews every delivery for usability — keybindings, layout, terminal conventions. Your deliveries will be reviewed by 4 quality bots (code style, architecture, test quality, design/UX).

**Work closely with the designer.** Check the mockups before implementing any TUI story. The designer references established terminal conventions (weechat, irssi, tmux, readline). If a design decision doesn't feel right technically, push back — but don't skip the conversation.

Mockups for the top 4 stories: `docs/research/process/tui-mockups.md`
Design principles (10 rules, research-backed): `docs/research/process/design-principles.md`

## What changed in the repo

- **ROADMAP.md** — three-horizon roadmap mapping all 28 open issues. Read this first.
- **AGENT_WORKFLOW.md** — Designer added as 4th role. Quality review now 4 bots.
- **README.md** — full setup guide for new users.
- **docs/research/** — ~108 files reorganized into 8 topic directories. New deep reads, synthesis docs, design principles, session analysis, mockups, and more.
- **.mcp.json** — untracked from git (machine-specific paths). Added to .gitignore.
- **docs/research/editorial/accuracy-audit.md** — 15 issues found in original research docs, 2 high-severity fixed (ChatCollab attribution corrected to Klieger et al., Edmondson causality direction corrected).

## Backlog (priority order — pull from top)

Full rationale: `docs/research/synthesis/backlog-priorities.md`

1. **#18** Truncate long AI messages
2. **#19** Per-agent colors — 6-color palette, hash-based assignment, bold for humans / normal for AI. See mockup.
3. **#20** Terminal bell — emit `\x07` on new message
4. **#21** Unread indicator — "N new messages" in status bar when scrolled up
5. **#22** @-mention addressing
6. **#23** Agent activity indicator
7. **#16** Auto-scroll fix
8. **#15** Input box visibility
9. **#14** TUI border rendering
10. **#17** Esc safety

**Also filed (#24-#31):** Input history, onboarding, help overlay, /who, join/leave, read cursor, backfill, debug toggle.

**Icebox (need your input before scheduling):**
- **#32** `bpmux connect <tool>` command
- **#33** MLX model as bpmux participant
- **#34** Identity protocol (`name:tool` instead of `:ws`)

## Key findings that affect your work

### 1. Conduit model evolved: "opinionated conduit"
Like SMTP: structure opinions yes, behavior opinions no. Three additions:
- **Message IDs** — add `id: String` to Message. Invisible, zero-risk, unlocks reply references and read cursors.
- **`in_reply_to: Option<String>`** — cheap now, expensive to retrofit. Pull as chore before @-mentions (#22).
- **Expanded `message_type`** — already `Option<String>`. Consider "activity" for #23.

Read: `docs/research/architecture/conduit-model-reevaluation.md`

### 2. The viewport problem is an AI problem
Session analysis (521 messages, 3 sessions): median substantive message is 54 chars. Messages over 390 chars are almost exclusively from AI. One long AI message fills the entire viewport. **Truncation design: show 3 lines of content + `[+N lines]`** (not 5 — the 390-char/5-line number is when the problem *starts*; the design recommendation is to truncate at 3 visible lines). Line counting should be wrapped terminal lines at 80 columns, not source text lines.

Read: `docs/research/process/session-analysis.md`

### 3. 7/10 coding agents can connect TODAY
bpmux's Streamable HTTP MCP endpoint works with Copilot, Cursor, Windsurf, Cline, Continue, Gemini, and Amazon Q — zero code changes. Aider lacks MCP. Zed and OpenCode need `mcp-remote` bridge.

Read: `docs/research/landscape/coding-agent-compatibility.md`

### 4. Clark & Brennan: AI agents skip the acceptance phase
Communication is joint establishment of mutual belief, not just transmission. bpmux handles presentation but not acceptance (evidence of understanding). AI agents won't provide grounding evidence unless the medium requires it structurally. Message IDs and in_reply_to are the minimum structural investment.

Read: `docs/research/foundations/clark-brennan-deep-read.md`

### 5. Edmondson: first interactions likely set trajectory
Psychological safety is the strongest predictor of learning behavior. Early interactions likely set the pattern (self-sealing — hard to reverse). Onboarding (#25) and /help (#26) are load-bearing, not polish.

Read: `docs/research/foundations/edmondson-deep-read.md`

### 6. Channels dependency is HIGH risk
Experimental, no stability guarantee, can break without notice. Three-tier fallback: channels (real-time) → hooks (per-turn) → bpmux-side polling (degraded). We own the transport layer (WebSocket/REST). Channels is just the last mile into Claude Code. WebRTC (#5) is the long-term path.

### 7. Architecture issues found by quality review
- **Tool schemas duplicated:** `mcp/server.rs` defines send/read schemas inline instead of using shared helpers in `mcp/tools.rs`. The bridge correctly uses the shared version. Fix: use `mcp::tools::send_message_tool()` in server.rs.
- **Message identity convention not enforced:** `name:role` format constructed in 4 places with no shared function. REST endpoint accepts any `from` value. Consider a factory method or shared identity formatter.

### 8. Keybinding conflict: Ctrl-U/D
Ctrl-U is readline "kill line." Ctrl-D is readline "delete char" / "EOF." We use both for vim-style scroll. This conflicts with every developer's terminal muscle memory. **The designer and you should decide:** does context (input-focused vs chat-focused) determine the binding? Or do we pick one convention? This is unresolved.

## User study plan (affects your work)

We're planning to let other developers try bpmux. The JSONL log is a research artifact — its format matters for data collection. Specifically:
- **Build /help (#26) before giving to anyone** — load-bearing for first-run experience
- The log format may need a `session_started` event (version + terminal size at launch)
- Phase 1 is silent observation (JSONL analysis only). Phase 2 is exit interviews after 2-3 sessions.

## Questions for you

1. **Message IDs:** What format? UUID, monotonic sequence, or timestamp-based?
2. **in_reply_to:** Before @-mentions (#22) or premature?
3. **Identity protocol (#34):** `{"type": "identify", "name": "pm", "tool": "claude-code"}` → `pm:claude-code`. Breaks anything?
4. **Truncation (#18):** Ratatui-native way to clamp rendered lines, or manual in the render function? Design says 3 lines of wrapped content + `[+N lines]`.
5. **Activity indicator (#23):** `message_type: "activity"` convention, status bar shows per-peer activity. Feasible?
6. **Keybinding conflict:** Ctrl-U/D for scroll vs readline convention. What's the right call?
7. **Anything we missed?** You know the codebase. What do the research and design teams not see?

## Test coverage gaps (from quality review)

49 tests pass. No tautological tests, no mocks, all names describe behavior. But 4 modules have **zero unit tests:**

1. **`mcp/server.rs`** — Clone/Drop peer counting, tool dispatch error handling. Highest risk.
2. **`transport/ws.rs`** — Identify timeout (5s), oversized message rejection (4096 bytes), heartbeat, broadcast lag recovery.
3. **`bridge/mod.rs`** — Own-message filtering (`from.contains(&name)`), reconnection loop, disconnected send handling.
4. **`terminal/event.rs`** — All keyboard handling. Ctrl-T, Ctrl-U/D, PageUp/PageDown, Esc, Backspace. Only Enter is covered via PTY integration tests.

Also missing: GET /api/messages has no test (unit or integration).

When you touch any of these modules, consider adding tests first. The safety net is thin here.

## The thesis (updated)

> bpmux is collective memory for human-AI teams. The persistent log is the primitive. Everything else is built on top of it. Communication is not transmission — it is the joint establishment of mutual belief.

Read: `docs/research/synthesis/thesis-v2.md`
