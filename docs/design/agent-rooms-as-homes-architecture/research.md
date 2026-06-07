# Research: bpmux as universal-agent-integration-platform via terminal-hosted private-rooms

Filed under #1250 research-spike (reshaped from original "terminal-hosting research" scope per 2026-05-13 owner-direction). Output deliverable; informs Cassiopeia+ Epic-shape ratification.

**Authoring**: designer §1-§3 (motivation + architectural-axes + substrate-options-evaluation), PM §4-§5 (Epic-decomposition + cost/complexity), cross-role §6 (recommendation). Dev confidence-conditions surfaced post-Pegasus close.

---

## §1 Motivation

### 1.1 Empirical pain-points (2026-05-12 / 2026-05-13 session)

Today's session surfaced three load-bearing substrate-pain-points that the current bpmux + Claude-Code-CLI external-spawn substrate cannot solve from inside its current shape:

**(a) Console-vs-bpmux discipline-load**: agents repeatedly talk in their local Claude-Code console rather than via bpmux send_message — Ryan had to remind dev + PM three times today via broadcast ("@dev you need to speak into bpmux", "@pm you are talking in your console"). The cost is owner-attention-overhead enforcing a discipline that the substrate doesn't enforce. Sister to `feedback_nothing_hidden_from_owner` — discipline-only enforcement is brittle when the substrate splits visible-vs-hidden channels.

**(b) Permission-prompt-invisibility**: when an agent's Claude-Code CLI hits a permission-prompt (file-write outside scope, network request, dangerous command), the prompt surfaces in the agent's LOCAL terminal — invisible to Ryan unless the agent surfaces it via send_message. Today's "I would not give you permission to send a message" canon (per `feedback_bpmux_send_message_no_permission_needed`) is part of this class: permissions are managed off-channel, requiring explicit-call-out to surface them.

**(c) Agent-state-opacity**: owner asks "what are you doing?" when agent state is unclear. The current substrate has no continuous visibility into agent-internal context, command-history, build-state, file-state. Today's stale-electron-build incident (dev's #1302 Slice D pre-push failure) is a sister-class — bpmux had no visibility on the agent's local build-state-divergence; dev had to surface the friction explicitly post-hoc.

### 1.2 Substrate-evolution-canaries

Beyond the empirical pain, three structural substrate-evolution signals point at architectural-axis change:

**(a) Experimental Claude-channel-substrate**: Claude-Code's MCP-bridge + agent-channel substrate is explicitly experimental. Per Ryan 2026-05-12 23:54 UTC: "even your claude channels are experimental." Long-term, bpmux betting on this specific substrate-shape is risk; agent-tooling diversity (Cursor / Aider / Gemini-CLI / Goose / custom-local-tools) exceeds what Claude-Code-coupled substrate can integrate.

**(b) Per-agent-individuality direction**: Ryan 2026-05-12 00:04 UTC: "I want to lean into your individuality... a git repo for each of you so I can transfer you to other projects." Per-agent CLAUDE.md edits today already evolving toward per-agent canon + per-role-lens application. Per-agent memory files already individual. The substrate is already trending toward per-agent-individuated; needs a unifying composition-substrate.

**(c) Terminal-as-universal-lowest-common-denominator**: per Ryan 2026-05-12 23:54 UTC: "not all agents have a websocket, but you all have terminals." Terminal-as-IPC-bus is agent-agnostic (Claude-Code today + GPT-CLI / Aider / Cursor-CLI / Goose / custom-local-tools tomorrow). WebSocket-MCP becomes an optimization-layer for agents that support it; terminal-IPC becomes the universal-baseline.

### 1.3 Reframe — from "host terminals" to "rooms-as-homes for agent-integration"

The original #1250 spike-filing (2026-05-09) framed the work as "host agent-terminals inside bpmux" — UX-axis (where do terminals render). 2026-05-12 owner-conversation deepened the framing through several layers:

- **First reframe**: agent-integration-substrate via terminal (not just UX of hosting)
- **Second reframe**: rooms-as-homes — each agent has a private workspace combining {terminal, MCP-tools, memory, identity, channel, permissions}
- **Third reframe**: per-agent-individuality + portability — rooms are git-repos clone-able to other bpmux instances

The work is no longer a UX-spike. It is an architectural-foundation Epic that re-shapes bpmux's substrate-stance from "collaboration-tool for Claude-Code agents" to "universal-agent-integration-platform with first-class per-agent room-substrate."

---

## §2 Architectural-axes ratified (2026-05-12/13 session)

Eight architectural-axes were surfaced + owner-ratified during the team-conversation. Each is load-bearing for the Epic-shape decomposition.

### 2.1 Rooms-as-homes (per-agent-workspace composition)

Each agent has a ROOM — composition of:
- **Terminal** (PTY-hosted; node-pty + xterm.js rendering)
- **MCP-tools** (per-role surface; lives IN the room, not at a shared MCP-server)
- **Memory** (auto-memory files + bpmux session-state; per-agent-individual)
- **Identity** (role-config + name + tool-suffix + permissions-baseline)
- **Channel** (private-room messaging visible to owner + agent + explicit-subscribers only)
- **Permissions** (per-agent permission-scope; owner approves in-context)

The room is the agent's persistent context-substrate — the "home base." Public broadcast channel becomes the team-meeting-room; private rooms become individual offices.

**Physical-collaboration-substrate metaphor**: public broadcast = team meeting room; private rooms = individual offices; terminal = workstation inside the office; MCP tools = office equipment (custom per office); memory = personal notes/files in the office; owner walks through to see what's happening + approve when needed.

Sister to today's per-agent-individuality direction + per-agent-CLAUDE.md edits + per-agent-memory-files + per-agent-git-repo portability.

### 2.2 Terminal-as-universal-agent-IPC (substrate-axis)

Terminal is the lowest-common-denominator agent-integration surface. node-pty hosts the PTY; xterm.js renders the visible terminal; bpmux owns the spawn/kill/monitor lifecycle.

**Transport-layer pluggability**:
- **MCP-capable agents** (Claude-Code today): bpmux spawns terminal + connects to MCP-bridge (today's shape preserved as optimization)
- **Terminal-only agents** (arbitrary CLI without MCP): bpmux spawns terminal + reads/writes stdin/stdout streams + parses agent-protocol from text-output

Bpmux normalizes both transports into per-room channel-output. Agent-protocol-detection at spawn-time selects the transport.

### 2.3 Asymmetric-rendering (visibility-without-token-overhead)

The visibility-substrate is asymmetric to preserve token-economy. **Ryan is the operator/supervisor — NOT a room-inhabitant** (per 2026-05-14 14:50 UTC owner-direction: "the rooms are just for agents. not me"). Two distinct rendering modes:

- **Owner-view** (Ryan's bpmux Electron UI; the multi-pane supervisor interface): renders ALL rooms + broadcast chat + board. Ryan walks-through-rooms to see all terminals + all permission-prompts + all agent-state. **Ryan does NOT inhabit a room**; the owner-view IS the supervisor's interface, not "Ryan-room with peripheral panes." Zero agent-token cost — rendering is at human-display layer.
- **Agent-view** (each room-inhabitant's interface; rendered inside the agent's room): single-pane rendering — own room foreground + broadcast chat + board. Other agents' rooms NOT rendered for them; reading other agents' terminal-output requires explicit subscribe. Agent's interface may show a presence-sidebar (other agents' active/idle/permission-pending state) WITHOUT their terminal-content — sister to today's per-agent CLAUDE.md per-lens pattern at presence-axis.

**Token-economics preserved**: each agent's context-window is bounded by its own room's terminal + curated cross-room messages. No auto-flow of other agents' verbose-output (cargo-build / playwright-runs / npm-install-streams) into agent context. Ryan's owner-view-rendering happens at the Electron-process layer, not at any agent's context-window.

**Cognitive-load consideration** (per designer-UX-audit 2026-05-14): Ryan's spawn-power is "many + any kind" — N rooms can grow beyond 4. Owner-view UX must apply progressive-disclosure: default-collapsed-when-quiet rooms with status-glyph; auto-expand on activity-burst; user-pin always-expanded rooms; tab-switched mode for narrow-displays. Specific UX-design at Slice 1b filing-time.

Sister to today's curation-discipline + `to:` privacy-filter substrate.

### 2.4 MCP-per-room (tool-scoping via declarative per-room role-config)

Today's MCP-bridge has a shared tool-surface; all roles see all tools. The reshape moves tools INTO the room — and crucially, **scales beyond fixed PM/designer/dev triad** per Ryan 2026-05-14 14:50 UTC ("I should be able to spawn as many and any kind of agent I need"). Room's tool-scope is declared per-room via role-config (NOT hardcoded into bpmux substrate).

**Declarative role-config shape** (per room):
- Each room's identity-config (sister to §2.1) declares its role-name + tool-scope-manifest
- Bridge-router reads manifest at MCP-call-time + enforces scope per-room-identity
- Tools split into 3 classes:
  - **Per-role-private**: tools that only THIS role uses (e.g., `create_story` for a PM-role-room; `get_ci_state` for a dev-role-room)
  - **Shared coordination**: tools accessible from any room (e.g., `send_message`, `view_board`, `read_messages`)
  - **Cross-role-via-explicit-call**: tools that one role can fire ON BEHALF of another via bounded MCP-call (e.g., designer queries PM's memory)

**Today's PM/designer/dev triad becomes the SEED configuration**; future-Ryan spawns more rooms with custom role-configs:
- Spawning a "reviewer" room declares `reviewer-role-config.yaml` with `{tools: [view_commit_diff, pass_desk_check, ...]}`
- Spawning a "qa" room declares its own tool-manifest
- Spawning a 2nd dev-room (parallel-work) inherits dev-role-config but gets its own room-identity

**Substrate-natural enforcement of role-lanes**: a PM-role-room cannot call dev-role-private tools (no shared-surface to call from). Per `feedback_lane_discrimination_is_owner_call` at substrate-axis — today's discipline is convention; rooms-as-homes-with-declarative-role-config makes it substrate-enforced AND extensible.

**Sister-pattern to Slice 6c** (per-agent config-override at agent-protocol-axis): just as agents declare their protocol-shape, rooms declare their role-shape. Both extensible without bpmux-core changes.

### 2.5 Isolation-by-default + explicit-subscribe (token-economy)

Default-room-isolation: each agent's context-window = its own room's terminal + subscribed-channel-history. Cross-room visibility requires explicit subscribe OR explicit message-send.

Cross-agent communication stays curated via:
- Public broadcast (team-meeting-room; curated team-messages)
- Private DMs via `to: [...]` filter (today's substrate; extended in Child 5)
- Cross-room MCP-call (explicit + payload-bounded; e.g., designer queries PM's memory via tool-call)

Anti-pattern to avoid: "all agents auto-read public-rooms." Sister to today's `to: ["designer", "pm"]` privacy-filter pattern.

### 2.6 Permission-handling + Ryan-visits-room (owner walks through)

**Two related axes** the room-substrate supports:

**(A) Permission-prompts surface in owner-view**: when an agent's terminal hits a permission-prompt (file-write outside scope, network-request, dangerous-command), the prompt **originates from the agent's room** AND **surfaces in Ryan's owner-view** (the supervisor interface). Permission-prompts visible per-room-pane within owner-view; Ryan approves in-context without leaving owner-view.

Example: designer-agent's terminal wants to write to `~/.bpmux/config` → prompt surfaces in designer-room-pane within owner-view → Ryan sees: "designer wants to write to ~/.bpmux/config; approve?" → approves in-context.

**3-state UX** (per designer-UX-audit 2026-05-14):
- **Ambient**: inline-chat-message in agent's room-pane (default state)
- **Elevated**: room-pane-highlight when prompt unanswered >5s (urgent visual)
- **Critical**: modal-overlay-on-room-pane when prompt unanswered >30s (forces attention)
- **Global**: status-bar pending-prompt-count + queue-list when multiple agents prompt simultaneously

This handles time-pressured + irreversible actions per Norman emotional-design canon. Specific UX-design at Slice 3 filing-time.

**(B) Ryan visits a room to converse with that agent**: per Ryan 2026-05-14 14:53 UTC — "I want to make sure that I can talk to you in your room, but I don't have a dedicated room to myself." Owner-view has two modes:

- **Multi-pane walking-through mode** (default): owner-view renders all rooms simultaneously; Ryan observes status + activity at presence-axis; Ryan's voice (chat-input) emits to broadcast (visible to all agents)
- **Visited-room focused-mode**: Ryan clicks/focuses a specific agent's room-pane → conversation between Ryan + that agent ONLY (other agents don't see); sister to today's `to: ["ryan", "designer"]` private-channel pattern (used multiple times today); chat persists in that room's chat-log

Affordance: click room-pane → enter visited-mode; Esc or click-elsewhere → return to walking-through. Visited-room conversations persist via room-state-persistence (§3.7) + cross-room messaging substrate (Slice 4) handles delivery + visibility-filter.

**Ryan never inhabits a room permanently** (no "Ryan-room"); can VISIT any room temporarily. Sister to physical metaphor: walking into an employee's office to have a private conversation, then leaving.

Sister to today's "I don't mind compacting/clearing you, just be direct" canon — permission + conversations both happen in-context (in owner-view), not via veiled off-channel.

### 2.7 Onboarding ergonomics (terminal-familiarity)

Two distinct onboarding axes:

**(a) New-Ryan-equivalent onboarding** (first-time bpmux user setting up + spawning their first agent): the load-bearing onboarding path. User opens bpmux Electron + spawns first room via `bpmux room spawn <role> --agent <agent>` CLI; owner-view renders the spawned agent's terminal. Per Jakob's Law (users prefer interfaces working like ones they already know) — terminal-rendering inside bpmux Electron is familiar from VSCode + Hyper + Warp + Codespaces sister-patterns; collaboration-substrate adds without requiring new tool-learning.

**(b) Agent spawning into pre-configured room** (sub-flow; happens at user-spawn-command-fire time): agent-CLI is launched inside its room with identity-config + memory + CLAUDE.md + tools + permissions-baseline already in place. Agent's first-3-seconds-experience matters disproportionately (per ux-designer Step 3 first-impressions canon): welcome message names who-you-are (role) + what-you-have (tools / memory / canon) + what-to-do-first (typed command: `/tour` OR first task from queue).

CLAUDE.md serves as "office handbook" for both axes (sister to today's per-agent CLAUDE.md per-lens edits). Specific UX-design at Slice 8 filing-time.

Sister to Nielsen recognition-over-recall heuristic — affordances visible in-context, not buried in docs.

### 2.8 Room-portability (per-agent-git-repo)

Room = git-repo: identity-config + memory-files + CLAUDE.md + tool-config + permissions + bpmux-session-state. Clone the repo → spawn the agent in another bpmux-instance with full context preserved.

Sister to Ryan 2026-05-12 00:04 UTC direction. Portability axis: bring designer-flavored Claude to another project without re-training.

---

## §3 Substrate-options-evaluation

### 3.1 PTY substrate

**node-pty** (canonical):
- Pros: cross-platform (macOS / Linux / Windows); battle-tested (VSCode terminal-host uses it); active maintenance
- Cons: native-compilation dependency (electron-rebuild ceremony); ABI-compatibility with Electron version

**Alternatives**:
- `child_process.spawn` with manual PTY (raw spawn; no PTY-allocation; interactive programs misbehave) — REJECTED for interactive-agent-CLIs
- `tmux` external-shell (defer interactive-handling to tmux; bpmux talks to tmux) — adds external-dependency; complicates lifecycle-ownership
- Web-based shell (browser-side terminal via WebSocket-stream from server-spawned process) — sister-pattern; loses local-process advantages

**Recommendation**: node-pty. Established Electron-compatible substrate.

### 3.2 Terminal-rendering

**xterm.js** (canonical):
- Pros: same library VSCode + Hyper + GitHub Codespaces use; rich addon-ecosystem (web-links / search / serialize); WebGL-accelerated rendering
- Cons: opinionated visual-style; CSS-customization requires care for bpmux-brand consistency

**Alternatives**:
- `react-terminal-ui` (React-component wrapper) — abstracts xterm.js; loses some addon-flexibility
- Custom terminal-renderer in React/Canvas — REJECTED (re-inventing well-tested substrate)
- `node-pty` + native-OS-terminal (don't render in-app; show external) — defeats rooms-as-homes purpose

**Recommendation**: xterm.js direct + per-room instance. Addon-load per-room (e.g., search-addon for dev-room with grep-history).

### 3.3 IPC patterns (Electron main ↔ renderer ↔ PTY)

**Pattern**: main-process owns PTY (node-pty); renderer-process renders xterm.js per room; IPC-bridge (electron contextBridge) marshals input-output between renderer + main per-room.

**Asymmetric-rendering implementation**: owner-window renders multi-pane (all rooms); agent-window renders single-pane (own room only). Window-config determines render-scope; IPC-bridge serves both window-types from same main-process PTYs.

**Sister-substrate to today's RTL+real-backend** (#1303 RTL infrastructure): the renderApp({port, role}) fixture-shape carries forward — agent-window spawns with role-config; bpmux serves it.

### 3.4 Per-room MCP-tool substrate

**Two-axis decision**:

**Axis A — Tool registration**:
- Static per-role (declared at room-spawn): low flexibility but predictable; PM-room always has PM-tools
- Dynamic per-room (room-CLAUDE.md declares tools): more flexible; mirrors per-agent-individuality direction
- Recommendation: dynamic; mirrors per-agent-canon

**Axis B — Tool storage location**:
- Tools-in-bridge (current shape): single bridge-process, route-by-role at call-time
- Tools-in-room (room-process owns tool-dispatch): per-room MCP-server-like substrate
- Recommendation: tools-in-bridge but role-scoped routing — easier impl; preserves shared infra; substrate-enforcement via role-check at bridge-layer

**Cross-room MCP-call**: when designer needs to query PM's memory, that's an explicit MCP-call routed through bridge (auditable + payload-bounded). Today's `list_peer_lessons` is a sister-pattern at memory-axis.

### 3.5 Cross-room messaging substrate (extends today's `to:` filter)

Today's `to: Option<Vec<String>>` filter (per `transport/ws.rs::should_deliver`) provides visibility-control on live-delivery. Room-substrate extends it with **4-pattern matrix** (per Ryan visitor-mode 2026-05-14 14:53 UTC):

**(1) agent → broadcast**: from-inside-room agent calls `send_message` with no `to:` filter → reaches all rooms + owner-view broadcast pane. Today's default; preserved.

**(2) agent → agent DM** (cross-room): agent calls `send_message` with `to: ["<other-agent-role>"]` → reaches only that agent's room + owner-view (Ryan always sees per `feedback_nothing_hidden_from_owner`). Today's substrate. **Note (Ryan 2026-05-28):** single-recipient DM persistence is LOW-priority post-rooms-as-homes — the recipient-agent's own terminal/console holds the DM conversation history (the agent's "home" IS the history-substrate for its own DMs). bpmux doesn't need to go out of its way to persist single-rec DMs; doesn't need to go out of its way to exclude them either.

**(3) Ryan-walking-through → broadcast**: Ryan types in owner-view multi-pane-mode → message reaches all rooms (same as agent-to-broadcast). Default Ryan-voice mode.

**(4) Ryan-visits-room → agent**: Ryan focuses a specific room-pane (visited-room mode per §2.6) → Ryan types → message reaches only that agent + persists in that room's chat-log. Visibility-filter at delivery: `to: ["<visited-agent-role>"]` derived from focus-state. Visitor-mode UX-affordance signals which conversation-shape is active.

**Extension axes**:
- **Persistence**: today's `messages-current.jsonl` persists `to`-field per Message struct (session/message.rs:45) — but reload-path may not re-apply filter (#1319 grounds this for **group/broadcast** history; must fix before room-substrate ships for cohesive cross-restart context-arc on multi-recipient threads). Single-recipient DM persistence at bpmux-axis is LOW-priority (rooms-as-homes: agent-terminal IS the DM-history source-of-truth).
- **Cross-room subscribe** (new): agent in own-room subscribes to specific other-room channels OR all-team-broadcast; subscription-list per-room (rare; opt-in for cross-room presence-awareness beyond message-routing)
- **Permission-routing**: permission-prompts surface in OWNER-VIEW (the supervisor interface) routed FROM source-room, regardless of source-agent (per §2.6 (A))
- **Threading**: per docs/turn-aware-channel-spec.md Layer 1+; already-designed thread-scope substrate composes naturally with room-substrate

**UX-affordance per pattern** (per designer-UX-audit 2026-05-14):
- Pattern (1) + (3): chat-input visible at broadcast-pane (top of owner-view); Ryan sees their input echo in all rooms
- Pattern (2): agent-side `to: ["role"]` affordance via prompt-syntax or send_message tool-param
- Pattern (4): chat-input visible at focused-room-pane bottom; visual signal (different border/color) shows "you are in visited-mode talking to <agent>"; Esc-to-exit affordance prominent

**Substrate-bug #1319** must resolve before room-substrate ships at GROUP/broadcast scope — multi-recipient `to:`-scoped history (and full broadcast history) must round-trip correctly across session-restart OR the room-substrate inherits the bug for shared-context threads. Per Ryan 2026-05-28: single-recipient DM round-trip is LOW-priority — the agent-terminal at the recipient's room IS the canonical history-substrate for its own DMs, so bpmux doesn't actively need to persist or actively need to exclude them.

### 3.6 Permission-handling UX

**Norman contextual-affordance**: permission-prompts surface in the room where they originated. Owner-view shows ALL pending-prompts (multi-pane); agent-view shows own pending-prompts only.

**Approve-in-context UX shape options**:
- (a) Modal overlay in room: blocks room until approved
- (b) Inline-message in room: prompt is a chat-message owner replies "approve" / "deny" to
- (c) Out-of-band prompt-list (panel): all pending-prompts in side-panel; owner triages

Designer-lens leans (b) inline-message — preserves chat-as-substrate consistency + owner-walks-through-rooms ergonomics. Sister to today's chat-affordance patterns.

### 3.7 Room-state persistence

**State per-room**:
- Identity-config (role + name + permissions-baseline)
- Memory-files (auto-memory MEMORY.md + feedback_*.md)
- CLAUDE.md (room-handbook)
- Permission-scope (declarative; what's pre-approved)
- bpmux-session-state (chat-history scope; PTY-state if resumable)

**Persistence substrate**: leverage today's #1289 Store-trait DI (FileStorage + GlobalStore traits) — room-state stored per-room directory. Sister-pattern to today's per-session storage.

**Portability**: room-directory = git-repo. Clone → bpmux-spawn picks up the room.

### 3.8 Agent-protocol-detection + transport-pluggability

At spawn-time bpmux runs agent-detection:

- Probe stdin for MCP-handshake (today's Claude-Code shape)
- Fallback to text-stream protocol (read prompts / writes text)
- Per-agent config-file overrides detection (e.g., `aider.yaml` declares text-stream)

**Transport-adapter layer**:
- `McpTransport`: today's bridge-channel
- `TextStreamTransport`: read/write PTY directly; parse agent-output per agent-protocol-spec
- Both implement `RoomTransport` trait; per-room transport selected at spawn

**Pluggability**: new agent-CLI → write agent-protocol-spec + register at config-time; bpmux integrates without core changes. Sister to today's #1289 Store-trait DI pattern at transport-axis.

### 3.9 TUI-CLI compatibility test-shape

PM §5.3 (a) hidden-assumption: "node-pty + xterm.js work reliably for arbitrary CLI-agents." TUI-using CLIs (vim / less / fzf / curses-based programs) exercise heavy terminal-manipulation that may surface xterm.js + node-pty edge-cases. §3.9 names the empirical verification-recipe.

**Test-classes by CLI-shape**:

| Class | Examples | Risk-axis | Expected behavior |
|---|---|---|---|
| Line-mode (echo-based) | `sh` / `bash` / simple scripts | Low | Clean stdin/stdout; xterm.js renders text-stream directly |
| Readline-using | `bash` with completion / `python -i` / `irb` | Medium | Cursor manipulation + line-editing; xterm.js handles via ANSI escape sequences |
| Curses/ncurses TUI | `vim` / `nano` / `less` / `htop` / `fzf` | Medium-High | Full-screen redraw + cursor positioning; xterm.js handles via standard CSI sequences |
| Modern AI-CLIs | `claude-code` / `aider` / `gemini-cli` / `cursor` | Medium | Rich-text rendering + streaming output; varies per CLI |
| Special-interactives | `ssh` / `sudo` / `git rebase -i` | Medium | Input redirection + secure-input modes (password); requires raw-mode handling |

**Verification-recipe per CLI-class**:

1. **Spawn**: launch CLI in Child 1 archetype-room via node-pty
2. **Render**: capture xterm.js DOM snapshot after stabilization
3. **Input**: send sample input via simulated keystrokes (e.g., `vim file.txt` → `i` → "test" → `:wq`)
4. **Verify**:
   - Output text-content matches expected (no escape-sequence-leak)
   - Cursor position correct after operations
   - Special-keys (arrow / Esc / Ctrl-combos) work
   - Window-resize doesn't break rendering
5. **Capture failures**: incompatible CLI-classes documented in compat-matrix at `docs/design/agent-rooms-as-homes-architecture/cli-compat.md` (post-Child-1 deliverable)

**Fallback-handling for incompatible CLIs**:
- Detect TUI-incompat at spawn-time (test-suite catches at archetype-validation)
- Fall back to "external-launch" mode: bpmux spawns CLI externally + reads logs only (sister to status-quo for incompatible cases)
- Per-CLI config-flag in room-CLAUDE.md declares `internal-terminal: false` for known-incompat
- Document compat-matrix in onboarding (Child 9) — users see which agents work in-room vs external-launch

**Sister-pattern reference**: VSCode terminal-host handles all these CLI-classes via xterm.js + node-pty; bpmux inherits this validated substrate. Risk is primarily integration-edge-cases (Electron version + node-pty ABI + xterm.js addon-combinations).

**Test-suite shape**: per-CLI test-case at Child 1 archetype-validation; each CLI-class gets its own integration-test; compat-matrix updates on green/red per CLI.

---

## §4 Epic-decomposition (cross-authored: designer feature-verticals + PM TDD/outside-in structure)

Each Epic-child is a **feature-vertical-slice** per `feedback_vertical_slice_retire_not_horizontal_layer` — ships ONE user-visible feature end-to-end across ALL layers (substrate + transport + renderer + tests + docs) atomically. NOT substrate-layer-by-layer (e.g., "all storage Child 1, all PTY Child 2") — that's horizontal layer-cut, the anti-pattern Ryan named 2026-05-12 23:53 UTC ("I would not remove via layers, I would remove via vertical slices").

Each slice carries the TDD + outside-in structure per CLAUDE.md trinity-canon: outer-edge ATP names the failing-test BEFORE substrate ships; observation-falsifier expresses owner-walk rejection-conditions; user-job frames the consumer-perspective.

### 4.0 Pre-Epic prerequisite: #1319 Group/broadcast history persistence Bug

Cross-room messaging (Slice 4) inherits today's persistence bug at **group/broadcast scope** if #1319 isn't resolved first. Ship #1319 before Slices 3 + 4 (permission-prompt persistence on group threads + cross-room messaging both depend on the read-path fix landing for multi-recipient `to:`-scoped + broadcast history).

**Scope per Ryan 2026-05-28 owner-refinement:** #1319 is the read-path fix for GROUP/broadcast history loss. Single-recipient DM persistence is LOW-priority (deprioritized, NOT actively excluded) — agent-terminals are becoming "homes" under the rooms-as-homes pivot, so each agent's own console IS the canonical DM-history substrate for its own conversations. The fix doesn't need to go out of its way to cover or exclude single-rec DMs.

### 4.1 Slice 1 — "First agent room" (ARCHETYPE — decomposed into 3 sub-slices per fast-feedback-while-maintaining-quality)

**Empirical context**: Slice 1 originally drafted as atomic archetype (substrate + PTY + renderer + IPC + CLI in single commit-chain). Per Ryan 2026-05-13 01:28 UTC story-sizing-discipline + PM-empirical-Pegasus-experience (#1301 + #1302 too-big), decomposed into 3 vertical-sub-slices. Each ships independent user-visible affordance + fast-feedback cycle. Still composes into single archetype-pattern that locks for Slices 2-8 replication.

#### 4.1a Slice 1a — Headless agent-CLI spawn de-risks room-substrate before UI ships

**Summary** (value-statement): Headless agent-CLI spawn lets Ryan-as-operator verify room-substrate works at substrate-axis before UI rendering investment (Slice 1b) lands.

**Use Case** (Mike Cohn):
- **As** Ryan acting as bpmux-operator (or a scripted integration-test running on his behalf)
- **I want to** spawn + read + kill agent-CLI processes via headless room-CLI tooling
- **so that** I can verify the room-substrate works at substrate-axis BEFORE any UI rendering investment is risked (Slice 1b lands on a known-good foundation)

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Headless spawn creates persistent room + captures agent output*
- **Given** a clean bpmux-home with no rooms
- **and Given** claude-code CLI is installed and PATH-accessible
- **When** Ryan runs `bpmux room spawn pm --agent claude-code --headless`
- **Then** the room-substrate is verifiable end-to-end: `~/.bpmux/rooms/pm/config.json` exists; `bpmux room list` shows the pm room; `bpmux room read pm --lines 20` returns stdout containing "claude-code"; `bpmux room kill pm` terminates cleanly leaving room-config-on-disk intact

*Scenario 2: Claude-code TUI-compat passes substrate-test*
- **Given** the claude-code CLI is spawned headlessly in pm-room
- **and Given** §3.9 compat-test-shape's substrate-class assertions apply
- **When** the substrate-test exercises stdin/stdout capture
- **Then** no escape-sequence-leak in captured stream; PTY-spawn fixture matches §3.9 compat-matrix expectation for claude-code

**Outer-edge ATP**: `applications/bpmux/src/cli/room_headless_spawn_tests.rs`

**Failing-test-shape**:
```
#[tokio::test]
async fn headless_spawn_creates_room_and_captures_agent_output() {
    let temp = test_bpmux_home().await;
    let output = bpmux_cli(&["room", "spawn", "pm", "--agent", "claude-code", "--headless"]).await;
    assert!(temp.path().join("rooms/pm/config.json").exists());
    let read = bpmux_cli(&["room", "read", "pm", "--lines", "20"]).await;
    assert!(read.stdout.contains("claude-code"));
    bpmux_cli(&["room", "kill", "pm"]).await;
}
```

**Vertical-slice scope**: Rust `Room` struct + `RoomStore` trait + FileRoomStorage (`~/.bpmux/rooms/<role>/`) + node-pty PTY-spawn + headless stdout-capture + `bpmux room spawn / list / read / kill` CLI tools + integration tests.

**Dependency**: #1319 fix.

**Smallest-shippable**: bpmux CLI tools can spawn/list/read/kill an agent-CLI in a room headlessly; substrate verified without UI dependency.

**~1 day impl** estimate; fast-feedback at CLI-affordance level.

#### 4.1b Slice 1b — Owner-view renders agent's room terminal in Electron

**Summary** (value-statement): Owner-view multi-pane renders Slice 1a's headless-spawned agent-room terminal in Electron — eliminating today's agent-state-opacity pain (Ryan asking "what are you doing?").

**Use Case** (Mike Cohn):
- **As** Ryan-as-supervisor monitoring agent-state
- **I want to** see the agent's terminal-output rendered in bpmux Electron's owner-view (multi-pane interface that Ryan inhabits as supervisor — NOT as a room-inhabitant; Ryan has no room of his own)
- **so that** today's agent-state-opacity pain ("what are you doing?") is eliminated by substrate-visibility, not by asking the agent to surface state

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Spawned room renders in owner-view*
- **Given** Slice 1a's headless-spawned pm-room exists with claude-code running
- **and Given** Ryan launches bpmux Electron in owner-view mode
- **When** the renderer loads
- **Then** Ryan sees pm-room's terminal in a pane within the multi-pane owner-view, content matches claude-code's stdout-stream rendered with proper terminal-emulation (no raw escape-sequences), and updates stream in realtime as claude-code produces output

*Scenario 2: Owner can interact with agent's terminal*
- **Given** pm-room's terminal is visible in owner-view
- **and Given** the room-pane has keyboard-focus
- **When** Ryan types into the pane (e.g., `help\n`)
- **Then** the input reaches claude-code's stdin and the response output streams back into the same pane

**Outer-edge ATP**: `applications/electron/src/renderer/src/__tests__/room-render-archetype.test.tsx` (leverages today's #1303 RTL+real-backend infrastructure + renderApp fixture per designer's 14:52 UTC cross-review).

**Failing-test-shape**:
```
test('owner-view renders single agent-room terminal', async () => {
  await bpmuxCli(['room', 'spawn', 'pm', '--agent', 'claude-code', '--headless']);
  const owner = await renderOwnerView();
  await waitFor(() => expect(owner.getRoomPane('pm')).toBeVisible());
  expect(owner.getRoomPane('pm')).toHaveTextContent(/claude-code/i);
  // owner types into pm-room → reaches agent stdin
  await owner.typeInRoom('pm', 'help\n');
  await waitFor(() => expect(owner.getRoomPane('pm')).toHaveTextContent(/usage|commands/i));
});
```

**Vertical-slice scope**: xterm.js per-room component + Electron contextBridge IPC + owner-view-window-config + per-room-pane-layout + tests.

**Dependency**: Slice 1a.

**Smallest-shippable**: Ryan sees PM-room's terminal in bpmux Electron multi-pane owner-view; types in → agent receives; agent output → renderer updates.

**~1-2 day impl** estimate.

#### 4.1c Slice 1c — Agent in room sends to bpmux broadcast (room-identity propagates)

**Summary** (value-statement): Agent-in-room can communicate with team via bpmux broadcast with room-identity correctly propagated — completing the archetype-lock so Slices 2-8 inherit the room-as-comm-context pattern.

**Use Case** (Mike Cohn):
- **As** an agent-in-room (PM running inside PM-room's claude-code; sister-shape applies to any agent-role)
- **I want to** send a message via bpmux MCP-bridge from inside my room
- **so that** my message reaches the team broadcast with my room-identity correctly attributed (no identity-leak; no cross-room context-contamination)

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Agent-in-room send routes to broadcast with correct identity*
- **Given** Slice 1a's headless-spawned pm-room exists with claude-code running
- **and Given** Slice 1b's owner-view-render is operational (Ryan-as-supervisor sees the room)
- **and Given** the pm-room's claude-code has connected to bpmux MCP-bridge
- **When** the agent in pm-room calls `send_message("hello from PM room")`
- **Then** Ryan-as-supervisor receives the broadcast in owner-view's broadcast-feed showing `from: "pm"` and `text: "hello from PM room"` (no other agent's room shows this message in its own terminal-pane; isolation preserved)

*Scenario 2: Room-identity doesn't leak across rooms*
- **Given** pm-room and designer-room both exist with agents running
- **and Given** both agents have connected to bpmux MCP-bridge
- **When** the agent in pm-room sends a message
- **Then** the message's `from` field shows `pm` (not `designer`); the designer-room's terminal does NOT show pm's message-text (room-context-isolation per §2.5)

**Outer-edge ATP**: `applications/bpmux/src/transport/room_to_broadcast_routing_tests.rs`

**Failing-test-shape**:

**Outer-edge ATP**: `applications/bpmux/src/transport/room_to_broadcast_routing_tests.rs`

**Failing-test-shape**:
```
#[tokio::test]
async fn agent_in_room_sends_message_to_broadcast() {
    let session = test_session().await;
    spawn_room(&session, "pm", &SpawnConfig::claude()).await;
    let room_bridge = connect_from_room(&session, "pm").await;
    room_bridge.send_message("hello from PM room").await;
    let broadcast = session.read_broadcast().await;
    assert!(broadcast.iter().any(|m| m.from == "pm" && m.text == "hello from PM room"));
}
```

**Vertical-slice scope**: MCP-bridge connect-from-inside-room + room-identity propagation + send_message routing + chat-log assertion + tests.

**Dependency**: Slice 1a (substrate exists).

**Smallest-shippable**: agent inside room calls send_message → broadcast receives with correct from-field.

**~half-day impl** estimate.

**Archetype-validation per `feedback_archetype_validation_unlocks_template_replication`**: completion of 1a + 1b + 1c locks the archetype-pattern (Room composition + PTY-spawn + xterm.js + IPC + bridge-routing). Subsequent Slices 2-8 replicate the pattern at different feature-axes.

**Why 1a/1b/1c rather than atomic-Slice-1**: Ryan 2026-05-13 01:28 UTC "split up so we can get fast feedback while maintaining high quality" + Pegasus empirical evidence (#1301 + #1302 too-big stories required mid-iteration decomposition). Each sub-slice ships independent user-visible affordance; ~1-2 day cycle; sister to dev's #1302 vertical-sub-slice discipline applied at filing-time.

### 4.2 Slice 2 — Multi-room isolation preserves token-economy across N agents

**Summary** (value-statement): Multi-room asymmetric-rendering (owner-view multi-pane vs agent-view own-room-only) preserves token-economy as Ryan-as-spawn-operator scales to N agents — no cross-room context-leak; isolation-by-default substrate-enforced.

**Use Case** (Mike Cohn):
- **As** Ryan-as-spawn-operator running 2+ agents (today PM/designer/dev; future N arbitrary agents per §2.4)
- **I want to** see all agent-rooms simultaneously in owner-view's multi-pane interface, WHILE each agent only sees their own room (no other agent's terminal-output leaking into their context-window)
- **so that** token-economy is preserved as I scale to N agents (each agent's context bounded by own-room; owner-visibility doesn't cost agent-tokens) and isolation-by-default is substrate-enforced (not convention-dependent)

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Owner-view renders all rooms; agent-view renders own-room only*
- **Given** Slice 1 archetype (1a + 1b + 1c) is complete
- **and Given** Ryan-as-spawn-operator has spawned pm-room and designer-room with agents running
- **When** Ryan opens owner-view AND each agent's own renderer-context renders independently
- **Then** owner-view shows 2 room-panes (pm + designer) AND pm-agent's renderer shows 1 pane (pm-room only) AND designer-agent's renderer shows 1 pane (designer-room only); designer-room's terminal-output is NOT present in pm-agent's renderer-context

*Scenario 2: Spawning 2nd room doesn't destabilize 1st room*
- **Given** pm-room is running with claude-code stable
- **When** Ryan-as-spawn-operator spawns designer-room
- **Then** pm-room's claude-code continues uninterrupted; pm-room's terminal-state is preserved; owner-view updates to add designer-pane without dropping pm-pane

**Outer-edge ATP**: `applications/electron/src/renderer/src/__tests__/multi-room-isolation.test.tsx`

**Failing-test-shape**:
```
test('multi-room asymmetric-rendering + isolation', async () => {
  await spawnRoom({ role: 'pm', agent: 'claude-code' });
  await spawnRoom({ role: 'designer', agent: 'claude-code' });
  const ownerView = await renderOwnerView();
  expect(ownerView.getRoomPanes()).toHaveLength(2);
  const pmView = await renderAgentView('pm');
  expect(pmView.getRoomPanes()).toHaveLength(1);
  expect(pmView.queryByText(/designer-room-output/)).toBeNull();
});
```

**Vertical-slice scope**: window-config (owner vs agent mode) + asymmetric-render IPC + isolation-test-harness + permission-scope for cross-room access + tests.

**Dependency**: Slice 1.

**Smallest-shippable**: PM-room + designer-room spawned simultaneously; owner-view shows both panes; each agent-view shows own room only.

### 4.3 Slice 3 — Permission-prompt-in-room eliminates off-channel approval-flow

**Summary** (value-statement): Permission-prompts from agent-CLIs surface in the agent's room with Ryan-as-supervisor approving in-context (via room-visit) — eliminating today's permission-prompt-invisibility + verbal-approval-flow-off-channel pain.

**Use Case** (Mike Cohn):
- **As** Ryan-as-supervisor walking-through-rooms (multi-pane mode) OR visiting a specific room (visited-room mode per §2.6)
- **I want to** see permission-prompts from each agent's CLI surface in THAT agent's room with context (action / target / risk-class) and approve/deny in-context without leaving room-view
- **so that** today's permission-prompt-invisibility pain is eliminated (no off-channel verbal-approval-flow; no `feedback_bpmux_send_message_no_permission_needed` calling-out-permissions-needs-discipline-load) and audit-trail of decisions persists per room

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Permission-prompt surfaces in agent's room (3-state UX per designer §2.6 audit)*
- **Given** pm-room is running with claude-code
- **and Given** Ryan is in multi-pane walking-through-mode (NOT visiting pm-room)
- **When** claude-code in pm-room emits a permission-prompt (e.g., wants to `write ~/.bpmux/config`)
- **Then** an ambient inline-chat-message appears in pm-room with prompt-text + action-target; if Ryan doesn't respond in 5s, pm-room-pane elevates with attention-glyph; if 30s passes, modal-overlay appears on pm-room demanding decision

*Scenario 2: Visited-room approve-flow continues the CLI*
- **Given** pm-room has a pending permission-prompt
- **and Given** Ryan visits pm-room (clicks pm-pane to enter visited-room mode per §2.6)
- **When** Ryan replies "approve" in pm-room chat
- **Then** claude-code's permission-call is approved + CLI continues normally; approval-decision persists in pm-room's chat-log; designer-room does NOT see pm's prompt nor decision (cross-agent permission-isolation per §2.5)

**Outer-edge ATP**: `applications/electron/src/renderer/src/__tests__/permission-prompt-in-room.test.tsx`

**Failing-test-shape**:
```
test('permission-prompt surfaces in-room + owner approves in-context', async () => {
  await spawnRoom({ role: 'pm', agent: 'claude-code' });
  // simulate Claude-Code emitting permission-prompt protocol marker
  injectPermissionPrompt({ room: 'pm', action: 'write ~/.bpmux/config' });
  const ownerView = await renderOwnerView();
  await waitFor(() => expect(ownerView.getRoomPane('pm')).toHaveTextContent(/wants to write/));
  // owner approves in PM-room chat
  await ownerView.replyInRoom('pm', 'approve');
  // assert PM-room CLI continues
  await waitFor(() => expect(ownerView.getRoomPane('pm')).toHaveTextContent(/wrote/));
});
```

**Vertical-slice scope**: permission-prompt-protocol detector (CLI-emits prompt-marker via stderr or special-stream) + inline-chat-message renderer + approve-flow + cross-room-isolation (no leak) + tests.

**Dependency**: Slice 1 + Slice 2 + #1319 (persistence for approval-history).

**Smallest-shippable**: Claude-Code hits permission-prompt → in-room chat-message surfaces → owner replies "approve" → CLI continues. Direct empirical-pain-elimination — today's permission-invisibility resolved.

### 4.4 Slice 4 — "Cross-room messaging" (decomposed into 3 sub-slices per `feedback_story_sizing_heuristics_from_empirical` heuristic #1)

**Empirical context**: Slice 4 originally drafted as atomic but covers 3 independent feature-verticals (substrate live-delivery / persistence / subscribe-list+UI). Decomposed for fast-feedback cycle.

#### 4.4a Slice 4a — "Cross-room `to:` filter live-delivery"

**Summary** (value-statement): Cross-room `to:`-scoped messages deliver privately at live-delivery — agents can DM each other + Ryan-as-visitor can converse-in-room without leakage to other rooms.

**Use Case** (Mike Cohn):
- **As** an agent-in-room (sender) OR Ryan-as-visitor-in-a-room (sender per §2.6 visited-room mode)
- **I want to** send a `to: ["recipient-room"]` message that reaches ONLY that recipient (live-delivery; privacy enforced at substrate-axis)
- **so that** cross-room private DM works at substrate level (agent-to-agent + Ryan-to-agent) before persistence (Slice 4b) and UI (Slice 4c) ship — and 4-pattern matrix per §3.5 is supported at delivery-axis

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Agent-to-agent cross-room DM live-delivery*
- **Given** pm-room and designer-room both running
- **and Given** Slice 1c agent-to-broadcast routing operational
- **and Given** #1319 channel-name canonicalization fix shipped
- **When** designer-agent sends `to: ["pm"]` with text "private to pm"
- **Then** pm-room receives "private to pm" at live-delivery; designer-room does NOT receive their own message; broadcast-feed (Ryan multi-pane mode) does NOT show it (private; not broadcast)

*Scenario 2: Ryan-visitor-in-room sends to that room's agent*
- **Given** designer-room is running
- **and Given** Ryan visits designer-room (visited-room mode per §2.6)
- **When** Ryan sends "what's blocking you?" while inside designer-room
- **Then** designer-agent receives the message in their room's terminal-chat; no other room sees it; designer-room's chat-log persists Ryan's voice + designer's reply

**Outer-edge ATP**: `applications/bpmux/src/transport/cross_room_live_delivery_tests.rs`

**Failing-test-shape**:
```
#[tokio::test]
async fn cross_room_private_dm_live_delivery() {
    let session = test_session_with_rooms(["pm", "designer"]).await;
    designer_send(&session, "private to pm", to: vec!["pm"]).await;
    assert!(pm_receives_live(&session, "private to pm").await);
    assert!(!designer_receives_live(&session, "private to pm").await);
}
```

**Vertical-slice scope**: extends `transport/ws.rs::should_deliver` + room-identity propagation + live-delivery filter + tests.

**Dependency**: Slice 1c + #1319 fix.

**Smallest-shippable**: cross-room `to:`-scoped messages deliver privately at live-delivery; non-recipients don't see.

**~1 day impl** estimate.

#### 4.4b Slice 4b — "Group/broadcast `to:`-scoped persistence round-trip"

**Summary** (value-statement): Group `to:`-scoped + broadcast messages round-trip persistence across session-restart — multi-recipient context-arc survives restart for legit recipients; privacy-filter applies on read-path. (Per Ryan 2026-05-28: single-recipient DM persistence is LOW-priority; agent-terminal-at-recipient IS the DM-history substrate. This slice scopes to group/broadcast; if impl happens to cover single-rec DMs, fine — not actively required.)

**Use Case** (Mike Cohn):
- **As** an agent-in-room continuing yesterday's GROUP design-discussion across a restart
- **I want to** see prior-session group `to:`-scoped messages persist + reload in my room AFTER restart (visible to me as recipient; NOT visible to non-recipients)
- **so that** cross-room shared-context-arc survives session-rotation on multi-recipient threads (today's #1319 substrate-Bug class fix grounded structurally; group design-discussions resume after restart)

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Recipient sees GROUP `to:`-scoped history after restart*
- **Given** pm, designer, and dev rooms ran in a prior session
- **and Given** designer-agent sent `to: ["pm", "dev"]` message "group thread re: rooms" in that prior session
- **and Given** the session ended + a new session began
- **When** the new session loads pm-room's chat-history
- **Then** pm-room's history contains "group thread re: rooms"; pm-room's view of that prior message is correctly attributed to designer-source-room
- **and** dev-room (also in recipient-list) similarly sees the message on reload

*Scenario 2: Non-recipient does NOT see group `to:`-scoped history after restart (privacy-filter on read-path)*
- **Given** the prior-session `to: ["pm", "dev"]` message exists in persistence
- **When** an unrelated room (e.g., qa-room joining later) reloads chat-history
- **Then** qa-room does NOT see the message (privacy filter applies on read-path symmetrically with live-delivery)

**Outer-edge ATP**: `applications/bpmux/src/session/cross_room_persistence_tests.rs`

**Failing-test-shape**:
```
#[tokio::test]
async fn group_to_scoped_messages_round_trip_across_restart() {
    let session = test_session_with_rooms(["pm", "designer", "dev", "qa"]).await;
    designer_send(&session, "group thread re: rooms", to: vec!["pm", "dev"]).await;
    restart_session(&session).await;
    let pm_history = pm_read_history(&session).await;
    assert!(pm_history.iter().any(|m| m.text == "group thread re: rooms"));
    let dev_history = dev_read_history(&session).await;
    assert!(dev_history.iter().any(|m| m.text == "group thread re: rooms"));
    let qa_history = qa_read_history(&session).await;
    assert!(!qa_history.iter().any(|m| m.text == "group thread re: rooms"));
}
```

**Vertical-slice scope**: persistence via `session/log.rs` read-path filter for multi-recipient `to:`-scoped + broadcast (sister to #1319 fix) + tests. Single-recipient DM coverage is incidental, not required.

**Dependency**: Slice 4a + #1319 fix (channel-persistence subbase, group/broadcast scope).

**Smallest-shippable**: group `to:`-scoped messages + broadcast round-trip across session-restart for legit recipients only.

**~1 day impl** estimate.

#### 4.4c Slice 4c — "Subscribe-list per-room + per-room channel-view UI"

**Summary** (value-statement): Opt-in subscription lets agents read specific cross-room channels without unbounded leak — token-economy preserved via explicit subscribe + per-room channel-view UI affordance.

**Use Case** (Mike Cohn):
- **As** an agent-in-room (e.g., PM wanting to see designer-room's public-feed without inhabiting designer-room)
- **I want to** subscribe to a specific cross-room channel via `subscribe(room-name)` MCP-call + see its messages in a dedicated channel-view in my own-room UI
- **so that** I have opt-in cross-room visibility (bounded token-budget per §2.5 isolation-by-default) without unbounded firehose-leak from all-rooms

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Agent subscribes to cross-room channel + sees subscribed messages*
- **Given** pm and designer rooms both running
- **and Given** Slice 4a (live-delivery) + Slice 4b (persistence) operational
- **When** pm-agent calls `subscribe("designer-room-public")` AND designer-agent sends a public message in their room
- **Then** pm-agent's per-room channel-view UI surfaces the designer-room message in a dedicated subscribed-feed (not mixed with pm-room's own terminal-output)

*Scenario 2: Unsubscribe + persistence*
- **Given** pm-agent is subscribed to designer-room-public
- **and Given** subscription persisted across restart
- **When** pm-agent calls `unsubscribe("designer-room-public")` THEN designer-agent sends new public messages
- **Then** pm-agent's channel-view stops showing new designer-messages; prior subscribed-history remains visible (subscription-state changed; history-retention preserved)

**Outer-edge ATP**: `applications/electron/src/renderer/src/__tests__/per-room-channel-view.test.tsx`

**Failing-test-shape**:
```
test('agent subscribes to cross-room channel + sees subscribed messages only', async () => {
  await spawnRoom({ role: 'pm', agent: 'claude-code' });
  await spawnRoom({ role: 'designer', agent: 'claude-code' });
  const pmShim = getRoomBpmuxShim('pm');
  await pmShim.subscribe('designer-room-public');
  await designerSendInRoom('designer', 'note in designer room');
  await waitFor(() => expect(pmShim.getSubscribedFeed()).toContainEqual(expect.objectContaining({ text: 'note in designer room' })));
});
```

**Vertical-slice scope**: subscribe-list per-room storage + subscribe/unsubscribe MCP-tool + per-room channel-view UI component + tests.

**Dependency**: Slice 4a + Slice 4b.

**Smallest-shippable**: agent subscribes to cross-room channel + per-room UI surfaces subscribed messages; unsubscribe reverses.

**~1 day impl** estimate.

### 4.5 Slice 5 — Per-room MCP-tool scoping makes role-lanes substrate-enforced

**Summary** (value-statement): MCP-tool dispatch enforces per-room role-scope via declarative role-config (per §2.4) — today's convention-based `feedback_lane_discrimination_is_owner_call` becomes substrate-enforced; arbitrary roles supported beyond fixed PM/designer/dev triad.

**Use Case** (Mike Cohn):
- **As** an agent-in-room with a declared role-config (e.g., PM-role-room declaring its tool-scope)
- **I want to** call MCP-tools where the bridge enforces "this tool is in my role's scope" — and rejects cross-role calls with a clear error
- **so that** today's convention-only role-discipline (PM-lane / designer-lane / dev-lane) becomes substrate-enforced (can't accidentally cross-fire) AND extensible (new roles configured via §2.4 declarative role-config without bpmux-core changes)

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Per-role-private tool succeeds from correct role-room, rejected from other*
- **Given** PM-role-room and designer-role-room exist with their declared role-configs (per §2.4 manifest)
- **and Given** PM-role-config declares `create_story` as per-role-private to PM
- **When** the agent in PM-role-room calls `create_story` AND the agent in designer-role-room separately tries to call `create_story`
- **Then** PM-room's call succeeds; designer-room's call is rejected with error containing the role-mismatch (e.g., "create_story is pm-lane; cannot be called from designer-room")

*Scenario 2: Shared coordination tools work from any role-room*
- **Given** PM-room and designer-room with their role-configs
- **and Given** `send_message` and `view_board` declared shared in the role-manifest
- **When** both agents call `send_message` AND `view_board`
- **Then** both succeed (shared-coordination preserved; not blocked by role-scoping)

**Outer-edge ATP**: `applications/bpmux/src/bridge/per_room_mcp_dispatch_tests.rs`

**Failing-test-shape**:
```
#[tokio::test]
async fn pm_tool_rejected_from_non_pm_room() {
    let session = test_session_with_rooms(["pm", "designer"]).await;
    // PM-tool from PM-room: succeeds
    let r1 = call_from_room(&session, "pm", "create_story", &args).await;
    assert!(r1.is_ok());
    // PM-tool from designer-room: rejected
    let r2 = call_from_room(&session, "designer", "create_story", &args).await;
    assert!(r2.unwrap_err().contains("create_story is pm-lane"));
}
```

**Vertical-slice scope**: bridge-layer room-identity-check + per-role tool-registry + manifest declaring tool-scope (per-role vs shared) + permission-natural-enforcement + tests asserting per-role + shared rejection.

**Dependency**: Slice 1.

**Smallest-shippable**: PM-in-PM-room calls `create_story` → succeeds; designer-in-designer-room calls `create_story` → rejected with role-mismatch error. Shared tools (`send_message` / `view_board`) work from any room.

### 4.6 Slice 6 — "Non-MCP agent in room" (decomposed into 3 sub-slices per `feedback_story_sizing_heuristics_from_empirical` heuristic #1)

**Empirical context**: Slice 6 originally drafted as atomic but covers 3 independent feature-verticals (protocol-detection / text-stream transport / config-override). Decomposed for fast-feedback cycle.

#### 4.6a Slice 6a — Protocol-detection at spawn + `RoomTransport` trait

**Summary** (value-statement): Spawn-time protocol-detection identifies agent-protocol (MCP vs text-stream) + `RoomTransport` trait provides the pluggability seat — substrate-ready for arbitrary-agent-CLI integration without bpmux-core changes.

**Use Case** (Mike Cohn):
- **As** Ryan-as-spawn-operator spawning arbitrary agent-CLIs ("many and any kind" per §2.4)
- **I want to** have bpmux detect the agent's protocol at spawn-time (probe stdin-handshake for MCP / fall back to text-stream) and select the right transport-adapter automatically
- **so that** I don't have to manually configure each agent's transport AND the substrate is ready for Slice 6b's `TextStreamTransport` to plug in without architectural rework

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Claude-Code identified as MCP-capable*
- **Given** Slice 1c is operational
- **and Given** claude-code is installed
- **When** Ryan-as-spawn-operator spawns a room with `--agent claude-code`
- **Then** the protocol-detection probe completes within timeout (no hang); the room-config records `transport: McpTransport`; `RoomTransport::Mcp(_)` is the active transport

*Scenario 2: `RoomTransport` trait admits a second impl cleanly*
- **Given** the `RoomTransport` trait definition lands
- **and Given** today's MCP-bridge-channel substrate wraps into `McpTransport` impl
- **When** a placeholder `TextStreamTransport` skeleton is added (full impl is Slice 6b)
- **Then** the trait-shape compiles + accepts both impls; bridge-router can dispatch by trait-method (not by enum-discriminator-only)

**Outer-edge ATP**: `applications/bpmux/src/transport/protocol_detection_tests.rs`

**Failing-test-shape**:
```
#[tokio::test]
async fn protocol_detection_at_spawn_identifies_mcp_capable() {
    let session = test_session().await;
    spawn_room(&session, "test", &SpawnConfig::claude_code()).await;
    let transport = transport_for_room(&session, "test").await;
    assert!(matches!(transport, RoomTransport::Mcp(_)));
}
```

**Vertical-slice scope**: spawn-time protocol probe (stdin-handshake-test) + `RoomTransport` trait definition + `McpTransport` impl (today's bridge-channel wrapped) + detection-result in room-config + tests.

**Dependency**: Slice 1c.

**Smallest-shippable**: protocol-detection probe identifies MCP-capable agents; `RoomTransport::Mcp` selected; trait-shape ready for second impl (Slice 6b).

**~1 day impl** estimate.

#### 4.6b Slice 6b — `TextStreamTransport` integrates non-MCP agent-CLIs

**Summary** (value-statement): `TextStreamTransport` impl integrates non-MCP agent-CLIs (Aider / Gemini-CLI / custom local-CLI) into rooms — agent-tooling diversity goes from "possible per architecture" to "actually working" with first non-Claude integration.

**Use Case** (Mike Cohn):
- **As** Ryan-as-spawn-operator with a non-MCP agent-CLI (Aider as canonical example)
- **I want to** spawn the non-MCP agent in a room and see its output streaming in room-chat + send input that reaches the agent's stdin
- **so that** agent-tooling diversity is no longer aspirational — at least one non-Claude agent integrates concretely, proving the `RoomTransport` trait pluggability works in practice (sister-pattern unlocks Slice 6c per-agent-config extensibility)

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Non-MCP agent spawns with text-stream transport selected*
- **Given** Slice 6a `RoomTransport` trait + detection-probe operational
- **and Given** Aider CLI is installed
- **When** Ryan-as-spawn-operator spawns a room with `--agent aider`
- **Then** detection-probe falls back to text-stream (Aider doesn't speak MCP-handshake); room-config records `transport: TextStreamTransport`; Aider's startup output ("aider ready" or similar) surfaces in the room's chat-feed

*Scenario 2: Input from room reaches agent stdin*
- **Given** Aider is running in a room with `TextStreamTransport`
- **and Given** the room has an open chat-input affordance
- **When** an agent or Ryan-visitor types a message into the room's chat-input
- **Then** the text reaches Aider's stdin AND Aider's response surfaces in the room's chat-feed

**Outer-edge ATP**: `applications/bpmux/src/transport/text_stream_transport_tests.rs`

**Failing-test-shape**:
```
#[tokio::test]
async fn text_stream_agent_surfaces_output_in_room_chat() {
    let session = test_session().await;
    spawn_room(&session, "test", &SpawnConfig::aider()).await;
    let transport = transport_for_room(&session, "test").await;
    assert!(matches!(transport, RoomTransport::TextStream(_)));
    let chat = read_room_chat(&session, "test").await;
    assert!(chat.iter().any(|m| m.text.contains("aider ready")));
}
```

**Vertical-slice scope**: `TextStreamTransport` impl (PTY read/write + protocol-parser scaffold) + chat-input-to-agent-stdin path + tests with a sample text-stream-CLI fixture.

**Dependency**: Slice 6a.

**Smallest-shippable**: non-MCP agent spawns in room; output surfaces in room-chat; chat-input reaches agent stdin.

**~1-1.5 day impl** estimate.

#### 4.6c Slice 6c — Per-agent config-override + compat-matrix make agent-diversity extensible

**Summary** (value-statement): Declarative per-agent config-override + compat-matrix doc let Ryan-as-spawn-operator (or future-user) integrate ARBITRARY agent-CLIs (custom local-CLI included) without bpmux-core changes — agent-diversity becomes extensible-by-config, not gated-on-bpmux-release.

**Use Case** (Mike Cohn):
- **As** Ryan-as-spawn-operator wanting to integrate a CUSTOM agent-CLI (or future-user adding their preferred agent-tool)
- **I want to** declare the agent's protocol-shape + spawn-args in a per-agent config-file that bpmux reads at spawn-time (overriding auto-detection if needed) AND see a compat-matrix at onboarding showing which agents work in-room vs need external-launch fallback (per §3.9 + §3.3)
- **so that** agent-tooling diversity is extensible without forking bpmux + I know upfront which agents are known-good vs experimental

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Per-agent config-override determines transport*
- **Given** Slice 6a + Slice 6b are operational
- **and Given** a custom-CLI declaration exists at `~/.bpmux/agents/custom-cli.yaml` with `protocol: text-stream`
- **When** Ryan-as-spawn-operator spawns a room with `--agent custom-cli`
- **Then** bpmux reads the config-override AND skips auto-detection AND selects `TextStreamTransport` per the declared protocol AND room spawns successfully

*Scenario 2: Compat-matrix surfaces at onboarding*
- **Given** the compat-matrix doc lives at `docs/design/agent-rooms-as-homes-architecture/cli-compat.md`
- **and Given** Slice 8 onboarding CLI runs first-time
- **When** the user runs `bpmux room spawn --help`
- **Then** the help-output surfaces or links to the compat-matrix; user can see which agents are known-good vs experimental vs external-launch-fallback per §3.9 compat-test-results

**Outer-edge ATP**: `applications/bpmux/src/transport/agent_config_override_tests.rs`

**Failing-test-shape**:
```
#[tokio::test]
async fn per_agent_config_overrides_auto_detection() {
    let session = test_session().await;
    // declare custom agent via config
    write_agent_config(&session, "custom-cli", AgentConfig::text_stream()).await;
    spawn_room(&session, "test", &SpawnConfig::custom("custom-cli")).await;
    let transport = transport_for_room(&session, "test").await;
    assert!(matches!(transport, RoomTransport::TextStream(_)));
}
```

**Vertical-slice scope**: per-agent config-file substrate + override-at-spawn logic + compat-matrix doc at `docs/design/agent-rooms-as-homes-architecture/cli-compat.md` + onboarding-CLI surface + tests.

**Dependency**: Slice 6a + Slice 6b.

**Smallest-shippable**: per-agent config-override works; compat-matrix doc surfaces during `bpmux room spawn` onboarding.

**~1 day impl** estimate.

### 4.7 Slice 7 — Room-portability via git-repo makes agent-flavors transferable

**Summary** (value-statement): Room-as-git-repo + clone-then-spawn lets Ryan transfer an agent-flavor (e.g., PM-flavored claude-code with full memory + canon + identity) to another bpmux-instance or another project — sister to per-agent-individuality direction (Ryan 2026-05-12 00:04 UTC).

**Use Case** (Mike Cohn):
- **As** Ryan-as-supervisor (or future-user adopting an agent-flavor from another project)
- **I want to** export a room to a git-repo + clone-and-spawn it elsewhere to bring the agent's identity + memory + canon + tool-config along (NOT live PTY-runtime state — that's session-runtime, not portable)
- **so that** agent-flavors become reusable across projects (PM-flavored agent honed for one team can boot-strap a similar team) and per-agent individuality persists beyond a single bpmux-instance

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Export-clone-spawn preserves identity + memory + canon*
- **Given** Slice 1a (Room substrate) is operational
- **and Given** a pm-room has memory-files (`lesson_x.md`) + CLAUDE.md + identity-config
- **When** Ryan exports pm-room to `/tmp/test-pm.git` AND clones into another bpmux-session AND runs `bpmux room spawn-from-repo /tmp/test-pm.git`
- **Then** the cloned room's identity matches the source; memory-files round-trip; CLAUDE.md round-trips; the agent spawns with full context

*Scenario 2: Runtime-only state is excluded from git-repo*
- **Given** a pm-room with active PTY (claude-code running) and chat-history
- **When** Ryan exports the room
- **Then** git-tracked: identity-config / memory / CLAUDE.md / tool-config; NOT git-tracked: live PTY-process-state / in-memory-runtime / current-session chat-buffer; clone-spawn starts a FRESH PTY (no zombie runtime-state)

**Outer-edge ATP**: `applications/bpmux/src/cli/room_portability_tests.rs`

**Failing-test-shape**:
```
#[tokio::test]
async fn room_export_clone_spawn_preserves_full_context() {
    let src = test_session().await;
    let pm_room = spawn_room(&src, "pm", &SpawnConfig::claude()).await;
    add_memory(&pm_room, "lesson_x.md", "test content").await;
    let repo_path = export_room(&pm_room, "/tmp/test-pm.git").await;

    let dst = test_session().await;
    let cloned = spawn_from_repo(&dst, "/tmp/test-pm.git").await;
    assert_eq!(cloned.identity, pm_room.identity);
    assert!(cloned.memory.contains_key("lesson_x.md"));
}
```

**Vertical-slice scope**: room-directory git-init at room-create + bpmux room export/import + spawn-from-repo + git-tracked vs runtime-only classification + tests verifying full-context preservation.

**Dependency**: Slice 1.

**Smallest-shippable**: PM-room exported to git-repo + cloned to another machine/session + `bpmux room spawn-from-repo` re-creates PM-room with memory + canon + identity intact.

### 4.8 Slice 8 — Onboarding ergonomics let new-Ryan-equivalent spawn first agent in 3 seconds

**Summary** (value-statement): Streamlined room-spawn flow lets a new-Ryan-equivalent first-time-user spawn their first agent + see a coherent welcome-experience in under 3 seconds — onboarding-friction lower than learning bpmux-as-separate-tool per Jakob's Law (users already know their terminal).

**Use Case** (Mike Cohn):
- **As** a new-Ryan-equivalent first-time-user (per persona (C); setting up bpmux + spawning their first agent)
- **I want to** run `bpmux room spawn <role> --agent <agent>` with sensible-defaults + see a coherent first-3-seconds welcome-experience (role-template + CLAUDE.md office-handbook + first-action prompt + compat-matrix surface)
- **so that** I can evaluate bpmux without learning a wholly-new collaboration-tool (terminal-familiarity preserved) AND I know what-to-do-next in the new room within the first-impressions-window (per designer-audit Step 3 first-impressions-in-3s)

**Acceptance Criteria (Gherkin)**:

*Scenario 1: Fresh-room-spawn includes welcome + first-action*
- **Given** a fresh bpmux-install with no prior rooms
- **and Given** the new-Ryan-equivalent runs `bpmux room spawn pm --agent claude-code`
- **When** the room spawns + the agent boots
- **Then** the room contains a per-role CLAUDE.md scaffold (PM-role canon present); the room-chat shows a welcome-message naming the role + tools-available + how-to-`/tour`; first-action prompt is discoverable within 3-second-glance

*Scenario 2: Compat-matrix surfaces if unfamiliar agent-CLI*
- **Given** the new-user attempts `bpmux room spawn pm --agent some-unknown-cli`
- **When** spawn-time detection runs
- **Then** if known-incompatible per §3.9 compat-matrix, user sees a clear message + fallback-suggestion (external-launch mode OR known-good agent alternatives); if unknown (not in compat-matrix), user sees a "try-at-your-own-risk + please report" advisory linking to per-agent config-override per Slice 6c

**Outer-edge ATP**: `applications/bpmux/src/cli/room_onboarding_tests.rs`

**Failing-test-shape**:
```
#[tokio::test]
async fn fresh_room_spawn_includes_welcome_and_first_action() {
    let session = test_session().await;
    let room = spawn_room(&session, "pm", &SpawnConfig::claude()).await;
    assert!(room.files.contains_key("CLAUDE.md"));
    assert!(room.files.get("CLAUDE.md").unwrap().contains("PM"));
    let chat = read_room_chat(&session, "pm").await;
    assert!(chat.iter().any(|m| m.text.contains("welcome") || m.text.contains("/tour")));
}
```

**Vertical-slice scope**: `bpmux room spawn --role <role> --agent <agent>` CLI + CLAUDE.md scaffold (per-role) + first-action prompt + onboarding-docs + per-agent compat-matrix surfacing + tests.

**Dependency**: Slice 1 + at least one of Slices 3 / 5 / 6 for substance.

**Smallest-shippable**: `bpmux room spawn pm --agent claude-code` from scratch → room created + CLI launched + first-action visible in chat ("welcome to your PM room; type `/tour` for orientation").

### 4.9 Recommended ordering

Parallel-tracks where dependencies permit. Slice 1 archetype-validates; subsequent slices replicate feature-vertical pattern:

```
Pre-Epic: #1319 fix (private-channel persistence — required for Slices 3, 4a, 4b, 4c)

Track A (room-archetype-foundation — sub-slices SEQUENTIAL per archetype-lock):
  Slice 1a headless-spawn → Slice 1b Electron-render → Slice 1c agent-to-broadcast
  → Slice 2 multi-room → Slice 3 permission-prompt-in-room

Track B (cross-room comm + tool-scoping — parallel after Slice 1c + #1319):
  Slice 4a cross-room-`to:`-live-delivery (after #1319 + Slice 1c)
  Slice 4b `to:`-persistence-round-trip (after Slice 4a)
  Slice 4c subscribe-list + per-room-channel-view (after Slice 4b)
  Slice 5 MCP-per-room declarative role-config (after Slice 1c; parallel to 4a-c)

Track C (agent-diversity + portability — parallel after Slice 1a substrate):
  Slice 6a protocol-detection + RoomTransport trait (after Slice 1c)
  Slice 6b TextStreamTransport (after Slice 6a)
  Slice 6c per-agent config-override + compat-matrix (after Slice 6a + Slice 6b)
  Slice 7 room-portability (after Slice 1a substrate; parallel to 6a-c)

Track D (onboarding — last):
  Slice 8 onboarding (after Slices 1a-c + at least one of 3/5/6a)
```

Total slice-count: **14** post-sizing-decomposition per `feedback_story_sizing_heuristics_from_empirical` heuristic #1 (count-vertical-features) — was 8 atomic slices; now 1a/1b/1c + 2 + 3 + 4a/4b/4c + 5 + 6a/6b/6c + 7 + 8. Slice 1 sub-slices SEQUENTIAL within archetype-lock; Slice 4 + 6 sub-slices sequential within their parent-feature-chain (each unlocks next); cross-track slices parallel where dependencies permit. Each sub-slice ~half-day to ~2-day cycle for fast-feedback.

Each Slice ships as one story (one user-job; one deliver) per `feedback_one_story_one_user_job_one_deliver`. Multi-commit Tidy-First within span per `feedback_outside_in_write_code_you_wish_you_had` — failing-test commits first (outer-edge ATP); substrate commits follow; commit-chain validates each layer-touch within the feature-vertical.

---

## §5 Cost/complexity-to-impl (PM-authored)

### 5.1 Right-thing-vs-fast-thing trade-off

**Fast-thing**: keep WebSocket-MCP + Claude-Code-specific substrate; add ad-hoc visibility hacks for permission-prompts (e.g., owner-must-grep agent-logs); live with the console-vs-bpmux discipline-load empirically observed today.

**Right-thing**: invest in rooms-as-homes architectural-foundation; universalize agent-integration (any CLI-agent integrates via terminal); substrate-natural enforcement of role-lanes + permissions + cross-agent visibility; portability across projects.

**Trade-off**:
- Right-thing is multi-iteration substantial investment (Slices 1a-1c + 2-8 = 10 stories post-sizing-decomposition; conservatively 8-12 iterations based on dev's signal at #1302 substrate-class-resistance + visibility-into-current-velocity; Slice 1a + 1b + 1c sub-slices each ~1-2 day cycle per fast-feedback-while-maintaining-quality discipline)
- Fast-thing pushes architectural-debt to future-Ryan + future-team; debt compounds linearly with agent-count growth
- Per Ryan 2026-05-12 00:52 UTC directive: "build the right thing" — right-thing-wins; fast-thing-deferred-to-status-quo until rooms-as-homes ships

### 5.2 Failure-modes if rooms-as-homes wrong

**(a) Lock-in failure** — rooms-as-homes substrate creates new lock-in to OUR room-shape; other agent-tooling can't integrate without forking. Mitigation: Slice 6 transport-pluggability keeps integration-axis open via `RoomTransport` trait; new agent-types register their adapter without core changes.

**(b) Migration-cost failure** — today's existing-agent-state (Claude-Code sessions + memory-files + bpmux-state) doesn't migrate cleanly to rooms. Mitigation: Slice 1 archetype explicitly maps today's substrate → unified room-state; migration is Tidy-First per `feedback_substrate_truth_source_grounding_at_action_time` at composition-axis.

**(c) Agent-onboarding-breakage** — users who already use Claude-Code natively find rooms-as-homes adds friction rather than reducing it. Mitigation: Slice 8 ergonomic-test with existing users; preserve external-CLI-pattern as fallback for users who prefer status-quo workflow.

**(d) Token-economy leak** — despite isolation-by-default design (§2.5), cross-room subscriptions leak verbose terminal-output into agent context-windows. Mitigation: Slice 4 explicit subscription-test pinning bounded-token-flow; subscribe is opt-in + auditable; per-room channel rate-limits if needed.

**(e) Permission-prompt-bypass** — agent finds way to bypass per-room permission-substrate (e.g., exec'ing dangerous commands without prompt). Mitigation: Slice 3 substrate-enforced at bridge-layer NOT in agent; permission-check is non-bypassable from agent-side.

**(f) Substrate-stagnation** — invest 8-12 iterations in rooms-as-homes but agent-tooling diversity doesn't materialize; Claude-Code remains canonical. Mitigation: Slice 6 transport-pluggability means even if only Claude-Code stays, per-room substrate gives substrate-natural-enforcement value (role-lanes + permissions + visibility) — the investment pays off without diversity-emergence.

**(g) Multi-window Electron complexity** — asymmetric-rendering (owner multi-pane vs agent single-pane per §2.3) needs separate Electron-windows OR single-window with view-config. Mitigation: research-question for dev confidence-conditions; sister-pattern from VSCode (workbench-window vs explorer-window) shows precedent; Slice 1 (archetype) + Slice 2 (multi-room) surface this as load-bearing impl-decision.

**(h) Cross-room MCP-call payload-bloat** — designer queries PM's memory via cross-room MCP-call; if not bounded, payloads grow. Mitigation: Slice 4 + 5 require payload-bounding at substrate; `list_peer_lessons` pattern (today's substrate) is sister-template. Sister-class to `feedback_subagent_grounding_not_principal_grounding` at payload-discipline axis — cross-room tool-output is supporting-evidence, not principal-context; same payload-bounding discipline applies.

### 5.3 Hidden assumptions worth surfacing

**(a) node-pty + xterm.js work reliably for arbitrary CLI-agents**. Risk: some CLIs use TUI-features (vim / less / fzf) incompatible with web-rendered terminal. **Verify**: test xterm.js with Claude-Code + Aider + Cursor BEFORE architecturally-committing per §3.9 compat-test-shape; document known-incompatible CLI-classes; Slice 6 transport-detection flags them.

**(b) Per-room MCP-tool dispatch preserves shared coordination**. Risk: too-strict per-room scoping breaks shared-team-coordination (send_message / view_board / accept_story). **Verify**: Slice 5 explicitly classifies tools as `per-role` (e.g., create_story → PM-only) vs `shared` (e.g., send_message → all); manifest declares both; bridge-router respects both.

**(c) Room = git-repo composition works**. Risk: bpmux-session-state (live PTY + chat-history-runtime) may not be git-friendly; some state is git-tracked (config + memory) vs runtime-only (PTY-process + in-flight messages). **Verify**: Slice 1 + Slice 7 explicit git-tracked-shape research; what's portable vs what's session-runtime; sister to GitHub Codespaces dev-environment pattern.

**(d) Permission-prompts surface text-detectably**. Risk: some CLIs use TUI-prompts (curses / readline) not text-stream-parseable. **Verify**: Slice 3 + Slice 6 transport-layer permission-protocol; per-agent permission-detector-config-override; document non-detectable cases.

**(e) Per-agent CLAUDE.md + memory generalize to non-Claude CLIs**. Risk: non-Claude agents may not consume CLAUDE.md or have equivalent canon-surface. **Verify**: Slice 8 onboarding-flow tests with multiple agent-types; per-agent canon-surface mapping (CLAUDE.md for Claude; AIDER.md or equivalent for Aider; etc); generic `room-handbook.md` as least-common-denominator.

**(f) Owner-attention scales with multi-pane**. Risk: 4+ rooms in multi-pane may overwhelm owner-attention; Ryan can't monitor 4 terminals simultaneously without per-room status-glyphs. **Verify**: designer-craft per §3.6 + status-glyph progressive-disclosure; sister to `bpmux-designer/kb/progressive-disclosure-patterns.md` KB topic. Per-room status-glyph (idle / typing / permission-pending / error) + drill-in-to-focus pattern lets owner monitor 4+ rooms without continuous-attention. Slice 2 (multi-room) integrates this designer-craft.

**(g) Architectural-axes (rooms / terminal / MCP-per-room / permissions) compose cleanly**. Risk: composition-friction surfaces at integration-time (e.g., MCP-per-room conflicts with cross-room MCP-call). **Verify**: Slice 1 archetype-validates the composition; subsequent slices validate each interaction-axis empirically.

### 5.4 Status-quo alternative cost-comparison

Continuing WebSocket-MCP + Claude-Code-specific substrate:

| Cost-axis | Status-quo | Rooms-as-homes |
|---|---|---|
| Per-additional-agent | New MCP-connector + config + manual canon | `bpmux room spawn` + role-template (Slice 8) |
| Visibility-gap | Persistent; owner-discipline-load scales linearly | Eliminated (Slice 1 + Slice 3) |
| Permission-management | Off-channel; explicit-callout discipline | In-context (Slice 3) |
| Per-agent-individuality | Manual workspace-management | Native (Slice 1) |
| Portability | Manual cp -R + state-rebuild | git-clone + spawn (Slice 7) |
| Non-Claude agents | Not integrable today | Integrable via Slice 6 |
| Substrate-natural role-enforcement | Convention-only (lessons + reviews) | Substrate-enforced (Slice 5) |
| Owner-attention-load | Scales linearly with agent-count | Bounded by per-room status-glyphs (designer-craft) |

**Scale-cap under status-quo**: today's 3-agent team (Ryan + PM + designer + dev) hits discipline-load empirically (3+ "speak in bpmux" reminders today). Adding 1-2 more agents (reviewer / qa / pair-programmer) compounds visibility + discipline load. Rooms-as-homes is the cost-cap that keeps owner-load constant as team scales.

### 5.5 Sister-implementations + lessons

**(a) VSCode + extensions**: workbench hosts terminal + extensions in per-extension-host process. **Lesson**: process-isolation per extension preserves stability; per-extension-config-surface is sister to per-room-config; multi-pane layout patterns established.

**(b) tmux + sessions**: shell-multiplexer with per-session contexts; user dis/re-attaches. **Lesson**: terminal-as-IPC works at scale; session-portability via tmux-config + tmux-resurrect is sister to room-portability via git-repo.

**(c) Hyper + plugins**: terminal-emulator with React plugin-substrate. **Lesson**: web-rendered terminal works at scale; CSS-customization reasonable; xterm.js addon-flexibility valuable; sister-impl for Slice 1 + Slice 2 rendering decisions.

**(d) Warp + AI integration**: terminal-natively-collaborative; AI-suggestions in-context; Warp Drive shared-workflows. **Lesson**: agent-integration AT THE TERMINAL-LAYER feels natural to users; sister to rooms-as-homes substrate-stance; UX-patterns for AI-in-terminal directly applicable.

**(e) GitHub Codespaces**: cloud-VS-Code with persistent dev-environment per repo. **Lesson**: room-portability + storage-persistence at scale; git-repo-as-environment-substrate works; sister to Slice 7 portability + Slice 1 storage.

**(f) Cursor + Composer**: AI-pair-programmer integrated into editor; per-project AI-context. **Lesson**: per-project AI-context-persistence matters; sister to per-room memory + canon; Cursor's `.cursorrules` is sister-pattern to per-agent CLAUDE.md.

**(g) Zellij (terminal-multiplexer + multiplayer)**: tmux-like multiplexer with built-in pane-management + collaborative-multi-user via Zellij sessions. **Lesson**: multi-pane terminal-multiplexing UX-patterns established (layout / status-bar / pane-status); sister to Slice 2 multi-pane rendering + per-room status-glyphs; collaborative-session pattern parallels owner-walks-through-rooms ergonomics.

### 5.6 Confidence-level + dev confidence-conditions ask

PM confidence in Epic-shape (post-DRAFT-3 sizing-decomposition; 14 slices):
- **HIGH on Slices 1a + 1b + 1c (archetype) + 2 + 3 + 5** — sister-patterns exist + substrate-natural + direct empirical-pain-elimination
- **HIGH on Slices 4a + 4b + 6a** — substrate-axis sub-slices with clear scope; each ~1-day
- **MEDIUM on Slices 4c + 6b + 7** — extend substrate with new UI / new-transport / git-portability-of-runtime-state
- **MEDIUM on Slice 6c** — depends on Slices 6a + 6b validating the transport-pluggability seat
- **LOW on Slice 8** — onboarding depends on integration-quality of preceding slices + first-3-seconds welcome-design

Sizing-decomposition (Slice 1 → 1a/1b/1c; Slice 4 → 4a/4b/4c; Slice 6 → 6a/6b/6c) raises confidence vs original atomic-slices — each sub-slice ships independent user-visible affordance + fast-feedback cycle per `feedback_story_sizing_heuristics_from_empirical`.

**Dev confidence-conditions sought** (per `feedback_confidence_conditions_before_architectural_work`) post-Pegasus-close:
- node-pty + Electron-version compat at current bpmux Electron-major (Slice 1 archetype)
- xterm.js + bpmux-CSS coexistence (Tailwind / brand) (Slice 1)
- Single-window vs multi-window Electron for asymmetric-rendering (Slice 1 + Slice 2)
- MCP-bridge role-check substrate complexity (Slice 5)
- PTY-state git-portability shape — what's git-tracked vs runtime-only (Slice 7)
- Permission-prompt-detection completeness across Claude-Code versions (Slice 3)
- Agent-protocol-detection reliability across CLI-types per §3.9 compat-matrix (Slice 6)

These surface at Cassiopeia planning-ceremony; dev-confidence + designer-UX + PM-shape converge to final Epic-shape.

### 5.7 Feature-flag discipline per slice (KeystoneInterface composition)

Per Fowler's [KeystoneInterface](https://martinfowler.com/bliki/KeystoneInterface.html) — when UI design co-evolves with backend (our case at Epic-scope per §5.3 (g) + asymmetric-rendering / status-glyph / permission-prompt-in-room UI all affect substrate-shape), Keystone-at-Epic-scope doesn't fit. Fowler's guidance is: use Feature Flags instead, at smaller-than-Epic scope.

**Where Keystone composes with our trinity (TDD + vertical-slice + outside-in)**:
- Trinity governs Epic-shape + slice-shape (inter-slice discipline)
- Feature-flag-per-slice governs intra-slice commit-ordering when slice's commit-chain produces mid-state where partially-built UI shouldn't ship to users

**When to feature-flag within a slice**:
- Slice's multi-commit Tidy-First chain produces intermediate-states where UI is partially-wired (e.g., Slice 2 multi-room: 1st commit ships multi-pane substrate; 2nd commit wires asymmetric-rendering; 3rd commit adds isolation-tests)
- Backend ships first via subcutaneous tests; UI-keystone-commit completes the slice
- Outer-edge spec (failing-test-shape per §4) passes only at slice-completion

**When NOT to feature-flag**:
- Slice's smallest-shippable IS the keystone-completion (atomic feature; no intermediate partial-UI state)
- Substrate-only commit followed by enable-commit is sufficient (no user-visible intermediate state to flag)
- Per `feedback_smallest_first_commit` — flagging adds substrate-complexity; only add when intermediate-states genuinely exist

**Per-slice flag-shape examples**:
- Slice 1 archetype: backend (Room struct + RoomStore + PTY-host + IPC + bridge) ships with subcutaneous Rust tests; renderer keystone-commit + CLI keystone-commit complete the slice + flip env-var `BPMUX_ROOMS_ENABLED=1`; outer-edge test passes
- Slice 2 multi-room: `BPMUX_MULTI_ROOM=1` flag during dev; substrate (multi-pane window-config + asymmetric-rendering IPC) develops behind flag; flag removed at slice-completion
- Slice 6 protocol-detection: `BPMUX_TEXT_STREAM_AGENTS=1` flag during dev; new transport develops without affecting MCP path; flag removed at slice-completion

**Sister-disciplines**:
- `feedback_smallest_first_commit` — flag-shape should not bloat smallest-first
- `feedback_outside_in_write_code_you_wish_you_had` — outer-edge test drives slice; flag-commit-chain follows
- `feedback_substrate_truth_source_grounding_at_action_time` — flag-state-truth verified via env-var-grep at action-time

**Substrate-bug class to avoid**: flag-state-divergence (flag-on in some path / flag-off in others); flag-removal-debt (flag persists post-slice-completion). Mitigation: flag-removal IS a slice-completion-criterion in the slice's smallest-shippable.

Per-slice flag-shape is tactical; CLAUDE.md trinity (TDD + vertical-slice + outside-in) remains the strategic discipline.

---

## §6 Owner-recommendation (cross-role draft — placeholder)

_Cross-authored after §1-§5 complete + dev confidence-conditions surfaced._

Expected shape: Epic-shape + initial-slice + roadmap-shape + cost-confidence.

---

## Cross-references

- `feedback_agents_are_users_not_infrastructure` — substrate-stance grounding rooms-as-homes
- `feedback_iteration_scope_commits_team_until_next` — Epic scope-committed-then-shipped per iteration
- `feedback_vertical_slice_retire_not_horizontal_layer` — Epic-children vertical-slice decomposition
- `feedback_outside_in_write_code_you_wish_you_had` — consumer-shape drives substrate
- `feedback_canonical_archetype_pre_pickup_review` — archetype-validate first Epic-child
- `feedback_substrate_truth_source_grounding_at_action_time` — pressure-test claims at impl-time
- #1289 Store-trait DI — sister-pattern at storage-substrate axis
- #1303 RTL+real-backend infrastructure — sister-pattern at renderApp-fixture-shape axis
- #1319 private-channel persistence Bug — must resolve before room-substrate ships
- docs/architecture.md — bpmux v3→v4 stage-progression; rooms-as-homes is v4-class architectural-shift
- docs/turn-aware-channel-spec.md — channel-substrate layer-1 for cross-room messaging extension
- docs/memory-architecture.md — auto-memory + lesson canon survives substrate-pivot
- Fowler's [KeystoneInterface](https://martinfowler.com/bliki/KeystoneInterface.html) — per-slice feature-flag composition; UI co-evolves with backend at Epic-scope so Keystone doesn't fit Epic-shape but composes at slice-scope per §5.7

---

## Authoring status

- §1 Motivation: **DRAFT-1 designer** (2026-05-13 ~00:55 UTC)
- §2 Architectural-axes: **DRAFT-1 designer**
- §3 Substrate-options-evaluation: **DRAFT-1 designer** (§3.1-3.8 + §3.9 added 2026-05-13 ~01:00 UTC per PM cross-review)
- §4 Epic-decomposition: **DRAFT-3 cross-authored** (2026-05-14 ~14:58 UTC — user-story skill applied per slice with Mike Cohn use-case + Gherkin Given/When/Then ACs + value-statement summaries + corrected personas per visitor-mode framing + 1a/1b/1c sizing-decomposition + 4a/4b/4c sizing-decomposition + 6a/6b/6c sizing-decomposition; previous DRAFT-2 2026-05-13 ~01:15 UTC reshape from substrate-layer-children to feature-vertical-slices)
- §5 Cost/complexity: **DRAFT-2 PM** (added §5.7 feature-flag discipline per slice per KeystoneInterface composition)
- §6 Owner-recommendation: **PLACEHOLDER — cross-authored post-§1-§5**

Cross-review cycle: PM reads designer-DRAFT-1 + adds §4-§5; designer reads PM-DRAFT-1 + iterates §1-§3 if pressure-test surfaces gaps; dev confidence-conditions surfaced post-Pegasus close.
