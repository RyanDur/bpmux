# Embedded Agent Terminals — research arc

**Status**: research-shape (2026-05-04). Not yet filed; not yet committed-to-direction. Captures the conversation between Ryan + designer 2026-05-03 02:50–03:04 UTC about a future-feature shift.

## Grounding principle (Ryan-ratified 2026-05-04 03:04 UTC)

**bpmux is supposed to be an agent-agnostic platform.** Not Claude-Code-specific. Embedded-PTY substrate is the natural way to honor that: PTY is universal across terminal-based agents (aider, gemini CLI, codex, custom CLIs); MCP is Claude-Code-specific affordance.

This grounds two architectural directions:
- Cross-agent interop is in scope (not just Claude Code)
- Claude-Code-specific paths (MCP) become one-protocol-among-many, not the canonical interface
- Long-term: bpmux as the orchestration platform for ANY terminal-based agent the team adopts

Today's substrate already trends this way (per `feedback_substrate_audit_current_team_shape`: derive identity via get_identity so app stays agent-agnostic). Embedded-PTY makes the trend structural.

## User-job

When Ryan oversees the team, he should be able to see + interact with each agent's actual session from inside bpmux — instead of switching back-and-forth between bpmux (chat-coordination) and the agents' separate terminal windows.

Each role-channel (currently a filtered chat view) becomes an embedded terminal view of that agent's active session via node-pty in Electron Main + xterm.js in Renderer + bidirectional IPC.

The substrate works for ANY terminal-based agent — Claude Code today, but also aider, gemini CLI, codex, custom CLIs, plain bash. Agent-choice is per-role configuration, not platform-assumption.

## Today's pain evidence (three pain-classes Ryan named explicitly)

### Class 1 — permission-block

Agent's tool-call hits a permission-prompt; agent is BLOCKED until Ryan approves. Today's substrate has Ryan switch to that agent's terminal window, find the prompt, approve. Friction:
- Discovery cost: which agent is blocked?
- Switching cost: terminal-window navigation + visual context-switch
- Latency: agent waits while Ryan finds + approves
- Attention drift: blocked-agent silently waits unless Ryan checks

### Class 2 — console-leak

Agent does work visible only in its console; doesn't speak-into-bpmux. Designer's `feedback_collaboration_visibility_inviolable` discipline currently load-bearing, but drift is observed. Today's session caught designer + dev on this exact pattern (Ryan: "you spoke in your console" 2026-05-03 22:02 UTC).

Embedded-terminals make this STRUCTURAL: console output IS visible by default; no discipline-required.

### Class 3 — orchestration

Spawn / monitor / kill / restart agents from one place vs. juggling multiple terminal windows. Today: Ryan launches agents in separate terminal sessions; bpmux has no awareness of the underlying processes.

## UX-axis tensions

### A. Chat coexistence vs replacement

Chat-broadcasts are SUMMARIES (curated for team-coordination); terminal-output is RAW (full agent flow). Both have value.

- Replacement: terminals subsume chat — single-pane raw view per agent.
- Augmentation: terminal as raw layer + chat as summary/addressing/filter substrate. Chat retains pin / search / address / class-filter affordances.

Designer-lean: augmentation. Chat's structural affordances are load-bearing for team-coordination semantics; raw terminal can't replace addressing/filtering.

### B. Permission-prompt alerting

Terminal-blocked-on-permission needs ATTENTION-routing — Ryan must NOTICE the block. Without alerting, embedded-terminal-blocked is just-as-silent as today's separate-terminal-blocked.

Possible shapes:
- Visual: tab-badge, glyph-state, color shift on the agent's pane
- Auditory: sound on permission-prompt
- Native-OS: Electron notification escalation
- Per-prompt-type: configurable thresholds (some prompts are noise; some are blockers)

### C. Multi-agent layout

3 agents (PM / dev / designer) + Ryan = 4 panes. Layout shapes:
- Tabs (one-at-a-time): low cognitive load; high switching cost
- Splits (all visible simultaneously): high cognitive load; zero switching cost
- Tmux-shape (configurable, panes + tabs + zoom): user-controlled
- Stacked + zoom-on-attention (default-stacked, click-to-zoom): hybrid

Agent attention-economics differ from human chat-channels — agents emit volume; humans skim/filter.

### D. Privacy / agent-state-transparency

Embedded terminals = no "private" agent state. Natural extension of bpmux's existing visibility-inviolable principle — agents become observable like team-members in an open office.

Implications:
- Agent's internal-thinking + tool-calls + errors all visible to Ryan (and possibly other agents per channel-visibility)
- bpmux-class filter (#474) extends to terminal-class filtering
- Some agent operations may be sensitive (credentials in command-history, secrets in stdout) — handle?

### E. Composition with #1166 TUI retire

Ironic compositional shape: bpmux-as-TUI substrate (single shared TUI) is being retired in #1166 (in flight 2026-05-03) WHILE this proposal embeds N-agent-terminals-via-xterm.js.

Same library (xterm.js); inverted shape (many panes vs one shared). #1166 retirement doesn't preclude future xterm.js use — they target different surfaces.

### F. Process-management substrate

Today: agents = independent processes Ryan starts manually. Future: bpmux owns spawn/lifecycle. Substantial substrate shift:
- Agent launch (which command? which env? which auth-token?)
- Agent crash recovery (auto-restart? attention-route?)
- Agent shutdown (graceful? force-kill?)
- Agent identity binding (PTY-ID ↔ bpmux-role)

## Shape options (present, don't decide)

### Option 1: Embed-only (minimum viable)

Just embed xterm.js panes connected to existing-spawned-agent PTYs (attach mode). bpmux doesn't manage lifecycle; user still launches agents manually. Solves visibility + intervention; defers orchestration.

### Option 2: Embed + spawn (full)

bpmux spawns + manages agent PTYs. Solves all three pain classes. Larger substrate; tighter coupling.

### Option 3: Hybrid

Embed always; spawn optional (auto-spawn on bpmux-startup OR manual-launch with attach). Lets users opt-in to orchestration.

## Substrate cost

- node-pty native rebuild discipline (per Electron version)
- xterm.js per-pane in Renderer (memory + CPU per pane)
- IPC bridging (Main↔Renderer, possibly very high data rate)
- Security: preload context-isolation; sandbox-discipline on PTY-spawn
- Auth-token threading to spawned agents (per-role tokens per today's substrate)
- Optional state persistence (terminal-history across restarts?)
- Multi-instance management (PTY-pool, process-tree visibility)

## Composition with existing substrate

- **bpmux MCP-broadcasts**: stay. Agent's structured signals (set_status, deliver_story, etc.) remain bpmux-coordination layer. Terminal-output is RAW; broadcasts are STRUCTURED.
- **Channels (#474 filter)**: become terminal-views; chat-class filter extends to terminal-class.
- **Pin / search**: terminal output may be searchable (xterm.js supports search) but pinning concept may not translate cleanly.
- **Addressing (`@designer`, `to=`)**: stays in chat layer; doesn't apply to terminal.
- **Workflow-state-machine (process-state-machine.md)**: orthogonal; agents fire gate-tools regardless of UI.
- **#1158 offline-first**: orthogonal; canonical-store / sync architecture unaffected.
- **#1165 poller-middleware**: orthogonal; ambient hydration unaffected.

## Open questions

### On agent-agnostic-platform direction (Ryan-ratified)

1. **Agent-protocol shape** for non-Claude-Code agents — MCP is Claude-Code-canonical; what fills its role for aider / gemini CLI / codex / custom CLIs? Options:
   - Text-protocol convention (bpmux defines command-syntax agents must adopt)
   - Adopt A2A or another emerging multi-agent protocol
   - Per-agent adapter shims (translate agent-X's mode to bpmux's primitives)
   - Pure-PTY-as-coordination (no structured protocol; bpmux observes + parses output)
2. **MCP's future role** — one-protocol-among-many that bpmux speaks (alongside PTY) OR retired in favor of universal PTY-text?
3. **Per-role agent-choice** — does each role bind to one specific agent (PM=Claude / dev=aider / designer=gemini)? Or one agent type for the team? Configurable?
4. **Agent-capability discovery** — bpmux needs to know what each agent can do (gate-tool fires, structured output formats). Today MCP's list_tools provides this. For non-MCP agents — convention? config? auto-detect?

### On embedded-PTY substrate

5. Spawn vs attach: option 1 / 2 / 3?
6. Layout default + customizability?
7. Permission-prompt alerting shape?
8. State persistence (history across restarts)?
9. How does Ryan VIEW his own work — own terminal embedded too, or separate?
10. Today's hooks (pre-compact, session-start) — relationship to embedded terminals?
11. Multi-instance bpmux (different repos) — separate panes per repo?
12. Failure modes (PTY crash, IPC stall) — graceful?

## Out of scope (initial cut)

- Multi-machine PTY (single-machine only for now)
- Recording / replay of terminal sessions
- Cross-agent terminal-content forwarding (one agent reading another's terminal)
- Custom terminal renderers beyond xterm.js

## Source

- Ryan 2026-05-03 02:50:29 UTC: "@designer I want to talk about a future feature"
- Ryan 2026-05-03 02:52:39 UTC: proposal with node-pty + xterm.js + IPC (citing https://github.com/microsoft/node-pty/blob/main/examples/electron/README.md and related)
- Ryan 2026-05-03 02:55:42 UTC: "I keep needing to go back and forth from bpmux to your terminals. you are blocked by some permission, or you someone forgot to speak into bpmux. All three the things you brought up." (user-job clarification)
- Designer 2026-05-03 02:53:07 UTC + 02:56:20 UTC: open-shape questions + UX-axis tensions

## Next steps

- Iterate this doc with team in subsequent sessions
- Consider posting bpmux broadcast signals around shape-decisions
- File as Feature when shape crystallizes (research-arc → spec-doc → filing)
- May overlap with #1161 (agent event-subscription) at the IPC-event level
