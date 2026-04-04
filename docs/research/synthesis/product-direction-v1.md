# bpmux Product Direction v1

Synthesized from Wave 1 (collaboration research, market research, design vision), Wave 2 (researcher critique of design, designer critique of research), and design principles. April 4, 2026.

This document separates what we know from what we believe, resolves tensions between the research and design perspectives, and proposes a prioritized build plan constrained by terminal reality.

---

## Section 1: What We Know (Research-Backed Facts)

### High Confidence

These claims have strong academic backing with multiple independent sources.

**Text chat has high grounding costs.** Clark & Brennan (1991) identify eight constraints on communication media. bpmux is strong on reviewability, sequentiality, cotemporality, and simultaneity. It completely lacks copresence, visibility, and audibility. The practical consequence: participants cannot use nonverbal backchannel signals to confirm understanding. Every message needs to be more explicit about intent, agreement, and understanding than it would be in voice or face-to-face. Silence is genuinely ambiguous -- it cannot be treated as agreement.

**Shared mental models degrade with team size.** Carraro, Furlan & Netland (2025) confirm that larger teams are less likely to share mental models. Mathieu et al. (2000) show teams with shared mental models perform better because members anticipate needs without explicit communication. The implication: bpmux's 3-participant sweet spot (1 human + 2 agents) is well within the manageable range. Scaling to 6-8 introduces a qualitatively different problem.

**Communication overhead grows quadratically.** Brooks (1975), validated by Chernoguz (2011) across 7,200 projects. For 3 participants: 3 channels. For 6: 15 channels. For 8: 28 channels. A single shared channel collapses this architecturally but not cognitively -- 8 voices in one stream is still 8 voices.

**Workspace awareness requires who/what/where.** Gutwin & Greenberg (2002) established that real-time groupware must provide awareness of identity (who), action and intention (what), and artifact focus (where). bpmux's current status bar provides only "who is connected" -- the most primitive awareness level.

**One driver at a time prevents coordination chaos.** Mob programming research (Zuill, 2014; Agile Alliance) identifies the driver/navigator/contributor pattern as the mechanism that makes groups of 3+ productive. The single-keyboard constraint eliminates coordination overhead of parallel work diverging. Pessimistic concurrency (locking) outperforms optimistic concurrency (merge-and-resolve) when conflicts are frequent and expensive -- which they are when multiple agents edit the same files.

**Agent transparency calibrates trust.** The SAT model (Chen et al., 2014; Endsley, 2017) demonstrates that when agents expose reasoning, goals, and projected outcomes, humans calibrate trust more accurately -- reducing both over-trust and under-trust. Lou et al. (2025) confirm that situational trust (how capable is this AI for this specific task) matters more than general trust.

**Collaboration is the #1 unmet need in AI dev tools.** GitHub's 2025 developer survey found only 17% of users agree agents have improved team collaboration -- the lowest-rated impact metric. Yet 4 out of 5 developers expect AI tools to make teams more collaborative. The gap between expectation and delivery is where bpmux sits.

**Cognitive and intent debt are now named concepts.** A March 2026 arxiv paper defines cognitive debt as "erosion of shared understanding" and intent debt as "absence of externalized rationale." Picked up by Martin Fowler (April 2, 2026) and Simon Willison. bpmux's JSONL log directly addresses both by externalizing agent reasoning.

### Medium Confidence

Supported by research but with caveats or limited validation in the specific bpmux context.

**Flat chat is correct for 3-6 participants.** Research on Slack threading (Speakwise, 2026) shows threading reduces noise but fragments awareness. For small teams, the awareness cost of threading outweighs noise reduction. However, this finding comes from human-only teams. The message volume and length from AI agents may change the calculus -- AI responses are often 10-20x longer than human messages.

**ChatCollab validates the shared-channel architecture.** Klieger et al. (2024) used Slack as a shared communication medium for mixed human-AI software teams. Event-driven, asynchronous turn-taking worked. Role differentiation emerged naturally. But ChatCollab used Slack (a mature platform with rich formatting) and studied teams with one configuration. Generalization to a raw terminal TUI is not established.

**Adjacency pairs improve multi-agent turn-taking.** Murder Mystery Agents (Frontiers in AI, 2025) found that self-selection with importance scoring, combined with adjacency pair norms (questions select their answerer), significantly reduced dialogue breakdowns. But this was in a structured game scenario, not open-ended software development.

**No one else is building the cross-tool message bus.** Market research confirms that every framework (CrewAI, LangGraph, AutoGen) is an orchestrator that owns the agents, and every IDE (Cursor, Windsurf) is a walled garden. bpmux is the cross-cutting shared channel. This is a positioning claim with current market evidence, but the market is moving fast -- any major player could build this.

### Low Confidence

Directionally plausible but lacking direct evidence.

**Enterprise compliance creates a buyer.** The EU AI Act requires audit trails for agent systems. bpmux's JSONL log could serve this purpose. But no enterprise buyer has been identified, and the gap between "could serve as an audit trail" and "meets procurement requirements" is enormous.

**A2A is the right next protocol.** Google's A2A handles agent-to-agent discovery and delegation. It would let any A2A-compliant agent join a bpmux session. But the designer critique correctly notes that no one evaluated whether A2A's spec is stable, whether real tools implement it in production, or whether the overhead is justified for a local dev tool.

---

## Section 2: What We Believe (Design Opinions with Partial Backing)

### The conduit model is the right architecture

**Belief:** bpmux is the message bus, not the interface. Users interact through their existing AI tools (Claude Code, Cursor, etc.). The TUI is the logger.

**Partial backing:** ChatCollab validated message-based collaboration through a shared channel. The market research confirms no one else occupies this niche. The conduit model is architecturally distinct from orchestrators.

**What would validate:** Successful multi-tool sessions where agents from different providers communicate through bpmux. A user who finds value in the cross-tool visibility that their individual tools cannot provide.

**What would invalidate:** Users consistently wanting to interact through the TUI directly rather than through their coding tools. Or AI tool vendors building native cross-tool communication, making the bus unnecessary.

### Two humans pairing with their AI teams is the real value proposition

**Belief:** The jump from 1 human + 2 agents to 2 humans + their respective agents is where bpmux becomes compelling for adoption.

**Partial backing:** None. The researcher critique is blunt: "No research supports mixed human-AI teams of 6-8 in a flat chat. The vision is building for a user that may not exist."

**What would validate:** Actually running a 2-human + 4-agent session and finding it productive. Even one successful session would be evidence.

**What would invalidate:** The session becoming an unreadable firehose. Participants losing track of who said what and why. The cognitive overhead exceeding the collaboration benefit.

### JSONL persistence is a superpower, not just a log format

**Belief:** The append-only JSONL log is both the audit trail and the team's externalized memory. Every platform feature (search, compliance, replay) is a view over this data.

**Partial backing:** Clark & Brennan's reviewability constraint is uniquely strong in bpmux. The cognitive/intent debt research gives this academic language. The EU AI Act creates regulatory demand for audit trails.

**What would validate:** Users referencing past sessions to inform current decisions. Session logs being useful to someone who was not present. An auditor accepting a JSONL log as compliance documentation.

**What would invalidate:** Users never reading back through logs. The raw log being too noisy to extract decisions from -- proving the researcher's point that "a 500-message JSONL log is not a shared mental model, it is an audit trail."

### Self-hosted first is the right deployment model

**Belief:** Enterprise customers want to run bpmux on their infrastructure. The product should be a binary you deploy, not a SaaS.

**Partial backing:** Developer tooling culture favors local tools. AI reasoning traces are sensitive data. Self-hosted is the trust-maximizing deployment model.

**What would validate:** Enterprise interest conditional on self-hosting. Users uncomfortable sending session data to a third party.

**What would invalidate:** Adoption blocked by operational burden. The designer's critique: "developer tools that require self-hosting have dramatically lower adoption than managed services."

---

## Section 3: What to Build Next

Ordered by impact/effort ratio. Research backing noted. All features constrained by terminal reality (80x24, single pane, status bar is one line, no mouse).

### Tier 1: Build This Week (high impact, low effort)

**1. Unread message indicator**
When scrolled up, show "N new messages" in the status bar. The data already exists (gap between scroll position and bottom). ~20 lines of code.
- Research backing: Grounding theory (Clark & Brennan) -- participants need to know when new information arrived. Workspace awareness (Gutwin & Greenberg) -- the minimum "what's happening" signal.
- Designer priority: #1 on the designer's list.

**2. Per-agent color differentiation**
Assign each unique `from` prefix a distinct color from a small palette instead of binary cyan/green. ~10 lines in the render function.
- Research backing: Trust calibration (SAT model) -- different agents need different trust levels, which requires visual distinction. The researcher flagged "agent messages are indistinguishable from each other" as a daily usability problem.
- Designer priority: #2 on the designer's list.

**3. Terminal bell on new messages**
Emit `\x07` when a message arrives and bpmux is not focused. One line of code. Solves the context-switching problem that no amount of in-TUI improvement can address.
- Research backing: Gloria Mark's interruption recovery research -- reducing the cost of context switching between editor and chat.
- Designer priority: #5 on the designer's list.

### Tier 2: Build This Sprint (high impact, moderate effort)

**4. @-mention addressing**
Parse `@name` in input text, render with highlight, enable the adjacency pair pattern. Convention-level feature, no protocol changes required.
- Research backing: Adjacency pairs (Sacks, Schegloff, Jefferson, 1974; MMAgents, 2025) -- questions select their answerer. Clark & Brennan -- explicit addressing reduces grounding cost in text media.
- Design principle: #3 (Address Messages Explicitly; Silence Is Not Agreement).
- Resolves tension: The researcher says turn-taking mechanisms are completely absent. The designer says this needs no protocol changes. Both are right -- @-mention is a rendering convention that enables a research-backed interaction pattern.

**5. Agent activity indicator in status bar**
Replace static "2 connected" with "2 connected | claude-1: thinking | claude-2: idle". Requires agents to send a lightweight status message type.
- Research backing: Gutwin & Greenberg's who/what/where framework. The researcher called this "almost mandatory for real-time collaboration."
- Terminal constraint: Must fit in one status bar line (~78 usable characters). For 2 agents, this works. For 5 agents, it does not -- would need a compact format or rotation.
- Designer priority: #3 on the designer's list.

**6. Input line editing (readline basics)**
Cursor movement (left/right arrow), word-delete (Ctrl+W), line-clear (Ctrl+U). Table stakes for a developer TUI.
- Research backing: Nielsen's consistency and standards heuristic (1994). Terminal conventions (GNU readline) are deep muscle memory.
- Design principle: #7 (Respect Terminal Conventions for Input and Editing).

### Tier 3: Build This Month (medium impact, significant effort)

**7. Message formatting (markdown-lite)**
Render basic structure from AI messages: bold for emphasis, dim for code fences, indented blocks for lists. Not full markdown parsing -- just enough to make multi-paragraph agent responses scannable.
- Research backing: Tufte's data-ink ratio principle. Calm technology (Case, 2015) -- move information between center and periphery of attention.
- Terminal constraint: The designer warns this is non-trivial in ratatui: "parsing, wrapping, code block detection, color management." The researcher calls it the "single highest-impact change" but the designer correctly flags rabbit-hole risk.
- Compromise: Start with code block detection only (fence with ``` renders in dim/alternate style). Defer bullet lists, headers, bold. Validate that code blocks alone improve readability before expanding scope.

**8. Keybinding help overlay (Ctrl+?)**
Toggle overlay showing available commands. One screen, dismissible.
- Research backing: Nielsen's recognition over recall heuristic. Progressive disclosure (NNG).
- Design principle: #9 (Make the First Minute Self-Explanatory).

**9. Session resume feedback**
On launch, display "Resumed session abc123, 47 messages" vs "New session abc123" as a status line.
- Research backing: Nielsen's visibility of system status heuristic.
- Design principle: #8 (Make Quitting Safe and Destruction Recoverable) -- the user needs to trust that resuming picked up where they left off.

### Tier 4: Validate Before Building (research gaps, uncertain value)

**10. Driver/navigator indicator**
Show which participant holds the editing role in the status bar. Transferable via command.
- Research backing: Strong (mob programming research, Zuill 2014). Both researcher and designer agree this matters.
- Why deferred: The current user (1 human + 2 agents) already coordinates through AGENT_WORKFLOW.md process rules. The indicator becomes critical only when a second human joins and the social protocol needs system support. Build when the multi-human scenario is validated.

**11. Grounding/acknowledgment mechanism**
Lightweight ack signals -- agents confirm understanding before acting. Possibly rendered as inline markers or status bar indicators.
- Research backing: Very strong (Clark & Brennan). The researcher called this "arguably the single biggest gap between the research and the vision."
- Why deferred: This is a protocol-level change that affects how agents behave, not just how the TUI renders. It requires designing the right signal (too heavy and it becomes noise; too light and it is ignored). Needs experimentation, not specification.

**12. Session state summary**
A compressed representation of "what have we decided, what are we doing, what is next" -- not just raw message history.
- Research backing: Strong (shared mental models literature, Mathieu et al. 2000). The researcher correctly identifies that "a 500-message JSONL log is not a shared mental model."
- Why deferred: The right implementation is unclear. Manual snapshots (/snapshot command) vs. agent-maintained running summary vs. auto-generated from log. Each has different costs and failure modes. Needs user feedback on what "catching up" actually looks like in practice before designing the solution.

---

## Section 4: Product Direction (Revised Vision)

### Horizon 1: Solid Daily Driver (6 weeks, revised from 1 month)

**Scope:** Tiers 1-3 above (items 1-9). The user is 1 human + 2 AI agents working on a single codebase.

**Revision from original vision:** Extended from 1 month to 6 weeks. The designer critique correctly flags that markdown rendering and input line editing are non-trivial. The original H1 was aggressive for a solo developer even with AI pair programmers.

**Cut from original H1:** Nothing cut, but markdown rendering is scoped to code-block-only as a first pass.

**Added to H1 (from research):** Per-agent colors, @-mention addressing, terminal bell, agent activity indicator. These were absent from the original vision but both research and design critique identify them as higher priority than some original H1 features.

**Success criteria:** A developer can use bpmux as their daily AI collaboration tool without friction. They can tell agents apart visually, direct messages to specific agents, know when new messages arrive while they are in another pane, and see basic formatting in agent responses.

### Horizon 2: Multi-Human Sessions (4-6 months, revised from 3 months)

**Scope:** Enable 2 humans + their respective agents in a shared session. This is the unvalidated bet -- build the minimum to test whether it works.

**Revised feature set:**
1. **Participant roster with role tags** -- the identity legibility that 4+ participants require.
2. **Session join tokens** -- minimum access control for sharing sessions across teams.
3. **Session snapshots for async handoff** -- the `/snapshot` command for timezone handoffs.

**Cut from original H2:**
- **Web UI (moved to H3).** The designer critique is right: "even a simple SSE + HTML read-only view requires a frontend, deployment consideration, and testing across browsers." For one developer, maintaining a web frontend alongside the Rust TUI is a significant context switch. The TUI is sufficient for active participants in H2.
- **Thread references / reply-to (deferred).** The researcher correctly notes this is a "half-measure" that adds metadata without solving the scanning problem. Defer until real multi-human sessions reveal whether flat chat actually breaks at 4-6 participants. If it does, the solution may be message tiering (design principle #6) rather than reply-to.

**Added to H2 (from research):**
- **Driver indicator in status bar.** With 2 humans, the social protocol for "who is editing" needs system support.
- **Grounding signals.** By H2, experimentation from H1 usage should reveal what form acknowledgment needs to take.

**The hard question:** The researcher says "no research supports mixed human-AI teams of 6-8 in a flat chat." The designer says this is where the value proposition lives. The resolution: build the minimum (roster, tokens, snapshots), run real sessions, and let the data decide. If 2-human sessions work, double down. If they do not, H2 becomes "better 1-human sessions with more agents."

**Anti-feature contradiction resolved:** The original vision rejected persistent identity in H2 but required it in H3 (org-scoped sessions with SSO). The revised direction acknowledges that participant identity is per-session in H2 with no accounts. If H3 enterprise features materialize, persistent identity gets designed then -- not pre-built now based on speculative demand.

### Horizon 3: Platform (9+ months, revised from 6+ months)

**Validated directions (build if H1-H2 succeed):**
- **Session recordings as searchable knowledge.** The JSONL index over completed sessions. Depends on sessions actually being useful to reference -- validate in H1/H2.
- **Agent provider abstraction.** The bridge contract abstracted from Claude-specific MCP bridge. Depends on demand from users of other AI tools -- validate by watching what tools people try to connect.
- **Read-only web observer.** Moved here from H2. The question "who specifically wants to watch a bpmux session passively?" needs an answer before building it.

**Speculative directions (interesting but unvalidated):**
- **A2A protocol support.** Promising for cross-tool agent discovery but the spec stability and real-world adoption are unverified. Monitor, do not invest.
- **Federated sessions across networks.** Engineering fantasy with no demonstrated demand. Keep as a directional idea.
- **Organization-scoped sessions with SSO.** Requires enterprise buyers who do not exist yet. Keep as a vision statement.
- **Session templates.** Solves a problem of workflow variety that does not yet exist. One known workflow (balanced team) does not need templates.

**Timeline revision:** Extended from 6+ months to 9+ months. The original H3 was directionally fine but packed features that are each products in themselves. The revision separates "validated, build when ready" from "speculative, build if evidence appears."

---

## Section 5: Open Questions

### Must Learn from Actual Usage

These questions cannot be answered by research. They require instrumentation or observation of real sessions.

1. **What is the human-to-agent message ratio?** If agents produce 10x the message volume, the TUI design needs to prioritize agent message compression over human message prominence. The current equal-weight rendering may be wrong.

2. **How long are agent messages in practice?** If agents routinely send 20-line responses, a single message fills the entire 80x24 viewport. Message collapsing or summarization may be more important than markdown rendering.

3. **Do users scroll back or stay at the bottom?** If users rarely scroll, the unread indicator matters less and the catch-up problem is different than assumed. If users scroll frequently, the "since you last looked" marker is critical.

4. **What triggers a human to check bpmux?** Proactive (routine check-in) vs. reactive (got a notification). This determines whether terminal bell / tmux alerts or better catch-up UX is the higher priority.

5. **Does the flat chat break at 4-6 participants?** The research predicts it might. The design insists it will not. Only real sessions can settle this.

6. **Is the JSONL log actually useful for catch-up?** Do users read back through the log to recover context, or do they just ask an agent "what happened?" If the latter, the log's value is as training context for agents, not as a human-readable artifact.

### Research Gaps That Remain

7. **No studies of terminal-based multi-agent chat.** All collaboration research assumes GUI environments with ample screen real estate. The 80x24 constraint changes which awareness mechanisms are feasible.

8. **No data on AI message volume in shared channels.** Slack research measures human message frequency (275 notifications/day). AI agents in a shared channel likely produce an order of magnitude more. The noise management strategies from human-team research may not transfer.

9. **No research on trust calibration across multiple AI agents in a coding context.** The SAT model addresses single-agent transparency. How a human calibrates trust differently between a PM agent and a tech lead agent in the same session is unstudied.

10. **No established patterns for grounding in human-AI text chat.** Clark & Brennan's framework predates AI entirely. What "explicit acknowledgment" looks like when one party is an LLM -- and whether it is even meaningful -- is an open question.

---

## Section 6: Anti-Features (Confirmed)

### Confirmed by Both Research and Design

**No threading.** Research: flat chat preserves shared awareness for small teams. Threading fragments it. Design: one pane, one stream, no structural complexity. For 3 participants this is unambiguous. For 6+, re-evaluate based on actual usage (see Open Question #5).

**No real-time collaborative editing.** Research: bpmux is a communication channel, not a shared artifact editor. Design: code editing belongs in the user's existing tools. Mob programming research confirms the "single keyboard" constraint is a feature, not a limitation. (Note: the researcher suggests a diff feed of agent changes rendered in the chat stream. This is worth exploring as a message type, not as editing infrastructure.)

**No agent runtime.** Research: every framework (CrewAI, LangGraph, AutoGen) is an orchestrator. bpmux is infrastructure, not orchestration. Design: managing agent lifecycle, resource allocation, and billing turns bpmux into a cloud platform -- a fundamentally different product.

**No project management features.** Research: the backlog lives in GitHub Issues. Workflow lives in process docs. Design: integrating with project tools (linking a session to an issue) is good. Reimplementing them is a trap.

**No general-purpose chat features.** Research: bpmux's signal-to-noise advantage comes from being scoped to a single session/task. Design: file sharing, voice/video, emoji reactions, app integrations -- these serve "chat in general" and dilute the product.

**No mobile client.** Design: the users are developers at workstations. A mobile view adds a platform to support without adding a user to serve. No research contradicts this.

### Confirmed with Caveats

**No themes or color customization -- for now.** The designer correctly flags that with 6+ participants in H2, two colors are inadequate. Per-agent colors (Tier 1, item #2 above) are not "customization" -- they are legibility. Full theme support remains an anti-feature. A fixed per-agent color palette is a necessary feature.

**No multi-pane layouts -- for now.** The researcher says a participant sidebar showing agent activity would address the workspace awareness gap. The designer says the 80x24 constraint makes this impractical. Resolution: the status bar is the awareness surface for H1. If H2 multi-human sessions need more, a minimal roster pane (5 characters wide, list of initials with status dots) could work without sacrificing meaningful chat height. Decide based on H2 usage.

**No plugin/extension points -- for now.** The researcher correctly identifies that the H3 "agent adapter layer" is a plugin system by another name. The design should not build plugin infrastructure in H1, but the bridge contract should be designed with future abstraction in mind. Not building plugins is correct. Building a tight coupling that prevents future plugins would be wrong.

**No persistent user accounts -- until evidence demands them.** Sessions are ephemeral, identity is per-session. The original vision contradicted itself by rejecting accounts in H2 and requiring them in H3. The revised direction: per-session identity through H2. If enterprise demand materializes, design persistent identity for H3. Do not pre-build.

---

## Appendix: Source Index

### Academic / Primary Research
- Clark & Brennan, "Grounding in Communication" (1991)
- Gutwin & Greenberg, "Workspace Awareness for Real-Time Groupware" (2002)
- Mathieu et al., "Shared Mental Models and Team Performance" (2000)
- Carraro, Furlan & Netland, "Shared Mental Models and Proactive Problem-Solving" (2025)
- Chen et al., Situation Awareness-based Agent Transparency model (2014)
- Lou et al., "Unraveling Human-AI Teaming" (2025)
- Sacks, Schegloff, Jefferson, "Turn-Taking in Conversation" (1974)
- Murder Mystery Agents, multi-party AI turn-taking (Frontiers in AI, 2025)
- Zuill, "Mob Programming" (Agile Alliance, 2014)
- Brooks, "The Mythical Man-Month" (1975); Chernoguz validation (2011)
- Nielsen, "10 Usability Heuristics" (1994)
- Amber Case, "Calm Technology" (2015)
- Tufte, "The Visual Display of Quantitative Information" (1983)
- Cognitive/Intent Debt paper (arxiv, March 2026); Fowler fragment (April 2, 2026)

### Industry / Market Research
- ChatCollab: Human-AI Software Teams (Klieger et al., 2024)
- GitHub Developer Survey 2025 (17% collaboration satisfaction)
- Slack Messaging Statistics (Speakwise, 2026)
- Cursor/Windsurf/Claude Code market analysis (various, 2026)
- Deloitte AI Agent Market Projections ($8.5B by 2026)
- A2A Protocol (Google/Linux Foundation, 2025-2026)
- MCP adoption data (97M monthly SDK downloads)
- EU AI Act enforcement timeline (2026)

### Design Research
- bpmux design principles document (10 principles with research backing)
- Progressive disclosure (Nielsen Norman Group)
- Information foraging theory (Pirolli & Card)
- WCAG 2.1 Success Criterion 1.4.1 (color accessibility)
