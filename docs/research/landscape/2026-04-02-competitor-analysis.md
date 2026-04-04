# Competitor Analysis: What We're Missing (and What We're Not)

*Researched April 2, 2026.*

---

## Validation: Unmet Demand Exists

**Cursor's community is actively requesting shared AI chat** — multiple forum threads asking for real-time shared workspaces where team members can view and contribute to AI conversations together. This is literally what bpmux provides.

**Windsurf claims "real-time collaboration"** but reviews note no actual real-time pair programming features. Strong for individual productivity, lacking team collaboration.

**Context loss is the #1 complaint** across all AI coding tools. Every new session starts from zero. Emerging solutions (OneContext, CleanAim, Erold) validate the problem. bpmux's persistent log with server-side read cursor directly addresses this.

## What to Learn From

### From Claude Code: Integration Surface
- bpmux as an MCP server is automatically available to subagents and Agent Teams
- Agent Teams' mailbox system is adjacent but different: single-machine/single-human/Claude-only vs bpmux's cross-machine/multi-human/tool-agnostic
- Document how bpmux interacts with hooks, subagents, and Agent Teams
- Consider a CLAUDE.md snippet teaching Claude when to read/write bpmux

### From Tuple/Pop: Pairing UX
- Near-zero latency is table stakes (WebRTC P2P achieves this)
- Zero-friction join via session code (already in spec)
- Keyboard-first design (TUI respects this naturally)
- E2E encryption (P2P + passphrase log encryption aligns)

### From Linear/Notion: Downstream Integration
- Pattern of AI-generated session summaries for import into tickets/docs (v1.x)
- Deep-linking issues to AI agents with assembled context (MCP integration, not bpmux core)

### From Zed: Multiplayer Editing
- Complementary, not competitive. Zed for shared editing + bpmux for shared AI reasoning

## What We're Correctly NOT Building

| Feature | Who Does It | Why Not bpmux |
|---|---|---|
| Code editing, diffing | Cursor, Windsurf, Zed | bpmux is a communication channel, not an editor |
| Screen sharing, video, audio | Tuple, Pop | Excellent tools exist; bpmux complements them |
| Issue tracking, backlog | Linear, Notion | bpmux surfaces reasoning; PM tools manage work |
| Multi-agent orchestration | Agent Teams, Gas Town | bpmux is the environment, not the orchestrator |
| Plan→implement→test pipeline | Copilot Workspace | That's the AI tool's job; bpmux is the channel |

**The v1 design's restraint is a strength.** Every tool that tried to be everything struggled (four failed Gas Town iterations, Cursor bolting on collaboration). The tools that succeeded were focused.

## Genuinely Worth Considering (v1.x+)

1. **Structured message references** — reference a file:line in a bpmux message, rendered as a link in TUI/Web UI. Tuple's "focus ring" equivalent for text chat.
2. **Session summary export** — after a session, generate a summary for CLAUDE.md, PR descriptions, or issue comments. Multiple tools validate this pattern.
3. **CLAUDE.md integration docs** — teach Claude Code users how bpmux's MCP server interacts with their existing setup.

## Sources

- [Cursor community feedback](https://forum.cursor.com)
- [Windsurf features](https://windsurf.ai)
- [Zed multiplayer](https://zed.dev)
- [Claude Code Agent Teams](https://code.claude.com/docs/en/agent-teams)
- [Tuple](https://tuple.app)
