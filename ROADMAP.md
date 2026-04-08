# bpmux Roadmap

**Thesis:** bpmux is collective memory for human-AI teams. The persistent log is the primitive. Everything else is built on top of it.

---

## Horizon 1: Polish the Dogfood

**Goal:** A developer who is not Ryan can use bpmux solo and have a good first experience.

**Timeline:** 8 weeks from start.

**Gate:** /help command works, no major TUI bugs, README explains the conduit model clearly.

### Milestones

1. **TUI reliability** ✅ -- borders, input, auto-scroll, message truncation all shipped
   - #14–#18 all closed

2. **Awareness** ✅ -- visual identity, notifications, unread counts, directed messaging, status dots
   - #19–#23 all closed. Status dots (#59, #67) shipped. Unread badge (#63) shipped.

3. **Onboarding** ✅ -- help overlay (#60), command input bar (#64), message history (#24)
   - #24–#26 all closed

4. **Session awareness** ✅ -- participant roster (#27), join/leave (#28)
   - #27–#28 all closed

5. **Transport reliability** ✅ -- cursor reads (#29), reconnection (#30, #73), system messages (#31)
   - #29–#31 all closed. Session handoff (#73) shipped.

6. **Channel navigation** ✅ -- sidebar tree view (#66, #71), /board (#69), channel switching (#54, #56)
   - All closed. Sidebar with ⌥1 toggle, three-state focus, unread counts.

7. **Message rendering** ✅ -- markdown (#80), code blocks (#61), syntax highlighting
   - tui-markdown integration with bold, headers (no hashes), syntax-highlighted code blocks.

### Active work (Horizon 1 polish)

- [#92](https://github.com/RyanDur/bpmux/issues/92) Channel awareness — agents know which channel the owner is talking in (verifying filter behavior)
- [#97](https://github.com/RyanDur/bpmux/issues/97) Bridge filtering — agents only receive relevant messages (token savings)
- [#98](https://github.com/RyanDur/bpmux/issues/98) Focus indicators for active panel
- [#100](https://github.com/RyanDur/bpmux/issues/100) Contextual shortcuts in status bar
- [#101](https://github.com/RyanDur/bpmux/issues/101) Type-to-filter channels in sidebar

**Recently shipped:** #79 (channel view), #81 (paste fix), #68 (status dots survive restart), #65 (status dots), #83 (/roadmap), #84 (clean startup), #85 (mouse clicks), #86 (text wrap alignment), #87 (activity filtering), #88 (large markdown), #90 (word wrap), #91 (broadcast banner), #93 (remove handoffs), #94 (remove pins), #95 (remove MCP endpoint), #96 (drop connected count)

**Research backing:** Edmondson (1999) -- first interactions disproportionately determine whether a tool feels safe or threatening. Gutwin & Greenberg (2002) -- minimum awareness requires presence, identity, and action visibility. Clark & Brennan (1991) -- reviewability and cotemporality are the constraints that matter most for text media.

**Resolved risks:**
- ~~Markdown rendering is a rabbit hole.~~ Solved with tui-markdown + syntect. One dependency, no custom parser.
- ~~Readline editing is non-trivial in terminal raw mode.~~ Shipped with crossterm + custom input handling.
- ~~Agent activity indicator requires a lightweight status message type.~~ Shipped via hooks (PreToolUse → busy, Stop → available).

---

## Horizon 1.5: Public Face and Memory

**Goal:** bpmux has a website, a migration story, and agents can rebuild theory on reconnect.

### Milestones

1. **Website/docs app** -- explains what bpmux is and why, surfaces research, provides OS-specific downloads
   - Reference pattern: battleship repo (React + Vite + GitHub Pages + GitHub Releases)
   - Not prescriptive — shares what we do and what worked, not a mandated process

2. **Arcs — walkable reasoning paths** -- the log captures teaching moments; arcs make them findable
   - Naur's theory building: theory can't be copied, only rebuilt through walkthrough. Arcs are asynchronous walkthroughs.
   - Experiments showed: log-only onboarding scores 4.5/5 on culture, one arc raises culture by a full point
   - Research backing: CBR (Schank), Experience Factory (Basili), conversation analysis (Sacks), Bohm's dialogue
   - Nature (base model) + nurture (arcs) = judgment. Neither alone is sufficient.
   - First real test: machine migration 2026-04-11

3. **Project health signals** -- glanceable board with Accelerate/DORA-aligned metrics
   - Throughput (stories accepted/cycle), cycle time (start to acceptance), rejection rate
   - Reference: Forsgren, Humble, Kim — measure flow, not output

---

## Horizon 2: Connect the Tools

**Goal:** Two developers using different coding agents collaborate through bpmux.

**Timeline:** 4-6 months after H1.

**Gate:** WebRTC peer connection working, at least 2 different AI tools connected, participants distinguishable by identity.

### Milestones

1. **P2P connectivity** -- real networking, not just localhost
   - [#5](https://github.com/RyanDur/bpmux/issues/5) Two developers can connect and chat in real-time (WebRTC)

2. **Identity and trust** -- who is who, and can you trust them
   - [#8](https://github.com/RyanDur/bpmux/issues/8) A developer knows who to trust in the chat
   - [#34](https://github.com/RyanDur/bpmux/issues/34) Participants identified by name and tool, not hardcoded suffixes (icebox -- see below)

3. **Multi-tool integration** -- any coding agent joins with one command
   - [#32](https://github.com/RyanDur/bpmux/issues/32) A developer can connect any coding agent to bpmux with one command (icebox -- see below)
   - [#33](https://github.com/RyanDur/bpmux/issues/33) A developer using a local MLX model can participate (icebox -- see below)

4. **Context management** -- sessions that scale
   - [#7](https://github.com/RyanDur/bpmux/issues/7) An AI respects the context budget

**Research backing:** Clark & Brennan (1991) -- cross-tool grounding costs are determined by the least-capable participant's constraint profile. ChatCollab (Klieger et al., 2024) -- validates shared-channel architecture under homogeneous conditions; heterogeneous conditions are uncharted. Cross-tool integration research confirms the lowest-common-denominator problem: a heterogeneous medium converges to its most limited participant unless the protocol compensates.

**Risks:**
- **Channels dependency.** Claude Code channels are experimental with no stability guarantee. The hook-based fallback must remain the reliable baseline. WebRTC (#5) is the path to real P2P that does not depend on Claude Code experimental APIs.
- **Heterogeneous grounding costs.** Different tools have different constraint profiles (push vs. poll, hooks vs. MCP-only). No research covers this scenario. bpmux will generate its own evidence.
- **Multi-human sessions are unvalidated.** No research supports mixed human-AI teams of 6-8 in flat chat. Build the minimum (roster, tokens, snapshots), run real sessions, let data decide.

---

## Horizon 3: Scale the Platform

**Goal:** Teams adopt bpmux with governance and compliance.

**Timeline:** 9+ months. Directional, not planned.

**Gate:** SSO or equivalent auth, audit trail export, `brew install bpmux`.

### Milestones

1. **Distribution and reliability**
   - [#9](https://github.com/RyanDur/bpmux/issues/9) bpmux deploys reliably (CI/CD, cross-platform builds, release automation)

2. **Security and compliance**
   - [#6](https://github.com/RyanDur/bpmux/issues/6) A developer's conversation is secure (DTLS, at-rest encryption, session tokens)
   - JSONL log as compliance infrastructure (EU AI Act 2026, audit trails)

3. **Accessibility**
   - [#3](https://github.com/RyanDur/bpmux/issues/3) A developer can see the chat in their browser (read-only web UI)
   - [#4](https://github.com/RyanDur/bpmux/issues/4) An AI can start a session without the developer touching a terminal

4. **Platform evolution**
   - A2A protocol adoption if 3+ major AI coding tools support it in production by H2 ship date
   - Session recordings as searchable knowledge base
   - Self-hosted-first with managed service option if demand appears

**Research backing:** EU AI Act enters enforcement 2026, high-risk agent systems require continuous audit trails, penalties up to 30M EUR. Enterprise AI collaboration market projected $8.5B (2026) to $35B. A2A protocol ecosystem stabilizing (Linux Foundation, Apache 2.0, co-founded by OpenAI/Anthropic/Google/Microsoft).

**Risks:**
- **Self-hosted vs. SaaS tradeoff.** Self-hosted limits adoption (GitHub vs GitLab, Slack vs Mattermost). Current architecture supports both. Do not pre-build managed infrastructure.
- **No enterprise buyer identified.** Gap between "captures the right data" and "meets procurement requirements" is enormous.
- **A2A is a delegation protocol, not a participation protocol.** It solves discovery, not group conversation. Useful for agent discovery only; bpmux's own WebSocket + MCP stack handles collaboration.

---

## Icebox

Issues that need team discussion before scheduling.

| Issue | What it is | What we need to decide |
|---|---|---|
| [#32](https://github.com/RyanDur/bpmux/issues/32) Connect any coding agent with one command | Generic bridge setup for Cursor, Copilot, etc. | What "one command" means when each tool has different MCP integration models |
| [#33](https://github.com/RyanDur/bpmux/issues/33) Local MLX model participates | Non-cloud AI joins a session | Whether the bridge contract is general enough or needs a new adapter pattern |
| [#34](https://github.com/RyanDur/bpmux/issues/34) Identity by name and tool, not hardcoded suffixes | Dynamic participant identity | Whether identity is session-scoped (H2) or persistent (H3), and what the message schema change looks like |
| [#72](https://github.com/RyanDur/bpmux/issues/72) bpmux-owned desktop notifications | Replace code-notify with native bpmux notifications | Focus detection (crossterm FocusGained/FocusLost) and OS notification integration |
| [#75](https://github.com/RyanDur/bpmux/issues/75) / [#76](https://github.com/RyanDur/bpmux/issues/76) WebRTC P2P | Spike + full story for peer-to-peer messaging | Rust library choice (datachannel, str0m, webrtc crate), battleship has Kotlin/Spring prior art |
| [#48](https://github.com/RyanDur/bpmux/issues/48) Auto-onboarding for AI | AI gets role + project context from session on join | Whether this is bpmux's job or the workspace CLAUDE.md |

---

## What We Will Not Build

| Anti-feature | Reason |
|---|---|
| **Threading** | Flat chat preserves shared awareness for small teams (Gutwin & Greenberg). Threading fragments it. Re-evaluate only if flat chat breaks at 6+ participants in H2. |
| **Real-time collaborative editing** | bpmux is a communication channel, not a shared artifact editor. Single-keyboard constraint is a feature (Zuill, 2014). |
| **Agent runtime / orchestration** | bpmux is infrastructure, not an orchestrator. Every framework (CrewAI, LangGraph, AutoGen) is an orchestrator. bpmux carries messages. |
| **Project management features** | Backlog lives in GitHub Issues. Workflow lives in process docs. |
| **File sharing, voice/video, emoji reactions** | General-purpose chat features dilute the product. |
| **Mobile client** | Users are developers at workstations. |
| **Persistent user accounts** | Session-scoped identity through H2. Accounts only if a paying enterprise customer needs them. |
| **Multi-pane layouts beyond sidebar** | Sidebar is the navigation surface. No additional panes — keep the TUI focused. |
| **Plugin/extension system** | Not now. But the bridge contract should not prevent future extensibility. |
