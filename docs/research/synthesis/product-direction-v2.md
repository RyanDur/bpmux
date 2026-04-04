# bpmux Product Direction v2

Revised from v1 incorporating Wave 2 critiques (researcher critique of design vision, designer critique of research), Wave 3 market research, and deep reads of primary sources (Clark & Brennan 1991, Gutwin & Greenberg 2002, Edmondson 1999). April 4, 2026.

**Through-line:** bpmux is collective memory for human-AI teams. The persistent log is the primitive. Everything else is built on top of it.

---

## Section 1: What Changed from v1

v1 attempted to synthesize research, design, and market findings into a three-horizon plan. The critiques exposed five structural problems:

1. **H1 was overloaded.** The researcher flagged that 9 features in 6 weeks for a solo developer is aggressive even with AI pair programmers. Several items (markdown rendering, readline editing) are non-trivial in a terminal TUI. [Source: researcher critique of design vision]

2. **Grounding was deferred but should be foundational.** v1 placed grounding/acknowledgment mechanisms in Tier 4 ("Validate Before Building") and called them "a protocol-level change that affects how agents behave." Clark & Brennan's deep read makes clear this is not optional -- it is "arguably the single biggest gap between the research and the vision." The acceptance phase of communication is as important as the presentation phase. [Source: Clark & Brennan deep read, Section 7; researcher critique]

3. **H2 was unfocused.** Web UI and reply-to were included despite weak justification. The researcher recommended cutting both. The designer flagged web UI as a significant context switch ("even a simple SSE + HTML read-only view requires a frontend, deployment consideration, and testing across browsers"). [Source: researcher critique; designer critique of research]

4. **Persistent identity was contradictory.** v1 rejected accounts as an anti-feature in H2 but required org-scoped sessions with SSO in H3. The contradiction was noted but not resolved. [Source: v1 Section 4, H2/H3 comparison]

5. **H3 treated A2A as speculative when market data supports it.** Market research confirmed A2A as the emerging standard for agent-to-agent communication, with Google donating it to the Linux Foundation under Apache 2.0, and the protocol ecosystem stabilizing around MCP (vertical: agent-to-tool) + A2A (horizontal: agent-to-agent). [Source: market research, MCP vs A2A protocol analysis]

---

## Section 2: The Persistent Log as Primitive

Before the horizons, the architectural thesis that governs all of them.

**Claim:** The append-only JSONL session log is the core primitive. Every feature -- awareness, grounding, compliance, knowledge management, catch-up -- is a view over this log.

**Evidence (high confidence):**

- Clark & Brennan identify reviewability as the property that distinguishes written media from spoken media. bpmux's log provides reviewability that "exceeds even email, because the log is a single ordered artifact rather than scattered across inboxes." [Source: Clark & Brennan deep read, Section 2, constraint #7]
- The cognitive/intent debt paper (arxiv, March 2026) defines cognitive debt as "erosion of shared understanding" and intent debt as "absence of externalized rationale." The log directly addresses both by externalizing agent reasoning. Picked up by Martin Fowler (April 2, 2026) and Simon Willison, indicating the concept has legs. [Source: v1 Section 1]
- Gutwin & Greenberg's past-tense awareness elements (action history, artifact history, event history, presence history) are all answerable from a structured log. [Source: Gutwin & Greenberg deep read, Section 1, Table II]

**Evidence (medium confidence):**

- The EU AI Act enters enforcement phase in 2026, with high-risk agent systems requiring continuous audit trails. Non-compliance carries penalties up to 30M EUR. bpmux's JSONL log could serve as compliance infrastructure. [Source: market research, enterprise AI collaboration findings]. The gap between "could serve" and "meets procurement requirements" remains enormous. No enterprise buyer has validated this. **Confidence: low for revenue, medium for positioning.**

**What would invalidate:** Users never reading back through logs. The raw log being too noisy to extract decisions from -- the researcher's point that "a 500-message JSONL log is not a shared mental model, it is an audit trail" holds until proven otherwise. [Source: v1 Section 2]

---

## Section 3: Horizon 1 -- Solid Daily Driver

**Timeline:** 8 weeks (revised from 6 weeks in v1, 1 month in original vision).

**Rationale for extension:** The researcher critique said H1 was aggressive. Two specific items are non-trivial: markdown rendering in ratatui ("parsing, wrapping, code block detection, color management" per designer) and readline-style editing (cursor movement, word-delete, line-clear). Adding a grounding mechanism -- which was deferred in v1 but should not have been -- further justifies the extension. [Source: researcher critique; designer critique]

**User:** 1 human + 2 AI agents working on a single codebase. This is us. Everything in H1 is validated by daily dogfooding.

### H1 Features (ordered by impact)

**1. Grounding mechanism (NEW -- promoted from v1 Tier 4)**

Lightweight acknowledgment signals so participants can confirm understanding before acting. Clark & Brennan's contribution model requires both a presentation phase (sending a message) and an acceptance phase (evidence of understanding). bpmux currently handles presentations but provides no structured mechanism for acceptances. "There is no read receipt, no acknowledgment signal, no way to know whether a message has been received, let alone understood." [Source: Clark & Brennan deep read, Section 7]

Implementation approach: Start with a convention, not a protocol change. Agents adopt a `[ack]` or `[understood: <summary>]` prefix pattern before acting on instructions. This is Clark & Brennan's "relevant next turn" -- the strongest form of acceptance evidence -- expressed as a message convention rather than infrastructure. If the convention works, it stays as convention. If it proves insufficient, H2 can add protocol-level read cursors.

**Confidence: high** that grounding is the right gap to close. **Medium** that a convention-level approach is sufficient. Clark & Brennan note that in text media with high speaker-change costs, "people try to do more within a turn" -- agents may naturally bundle acknowledgment into their responses rather than sending separate ack messages. [Source: Clark & Brennan deep read, Section 4, cost #8]

**2. Message truncation before markdown rendering (NEW)**

Long agent messages must be truncatable in the TUI before any formatting is applied. The designer critique flagged that markdown rendering is a rabbit hole; the researcher flagged that agent messages routinely fill the entire 80x24 viewport. Truncation with expand-on-demand is prerequisite to rendering, not the other way around.

Implementation: Show first N lines of any message exceeding a threshold (e.g., 10 lines). Keybinding to expand. This addresses Open Question #2 from v1 ("How long are agent messages in practice?") by making the answer irrelevant -- whatever the length, the TUI handles it.

**Confidence: high.** This is a daily usability problem observed in dogfooding.

**3. Unread message indicator**

"N new messages" in the status bar when scrolled up. The data already exists (gap between scroll position and bottom). [Source: Gutwin & Greenberg -- minimum "what's happening" signal; Clark & Brennan -- participants need to know when new information arrived]

**Confidence: high.** ~20 lines of code. Designer's #1 priority.

**4. Per-agent color differentiation**

Assign each unique `from` prefix a distinct color. [Source: SAT model (Chen et al. 2014) -- different agents need different trust levels, which requires visual distinction. Researcher flagged "agent messages are indistinguishable from each other" as daily usability problem.]

**Confidence: high.** ~10 lines in the render function.

**5. Terminal bell on new messages**

Emit `\x07` when a message arrives and bpmux is not the focused pane. [Source: Gloria Mark's interruption recovery research -- reducing context-switching cost]

**Confidence: high.** One line of code.

**6. @-mention addressing**

Parse `@name` in input text, render with highlight. Enables the adjacency pair pattern from conversation analysis. [Source: Sacks, Schegloff, Jefferson 1974; MMAgents (Frontiers in AI, 2025) -- questions select their answerer. Clark & Brennan -- explicit addressing reduces grounding cost in text media.]

Resolves a tension identified in v1: the researcher says turn-taking mechanisms are completely absent; the designer says this needs no protocol changes. Both right -- @-mention is a rendering convention that enables a research-backed interaction pattern.

**Confidence: high.**

**7. Agent activity indicator in status bar**

Replace static "2 connected" with "2 connected | claude-1: thinking | claude-2: idle". [Source: Gutwin & Greenberg -- Action element answers "What are they doing?" Called "almost mandatory for real-time collaboration." Currently no indication of activity between messages.]

Terminal constraint acknowledged: must fit in one status bar line (~78 usable characters). For 2 agents, this works. For 5+, would need compact format. [Source: designer critique]

**Confidence: high** for the awareness value. **Medium** for the implementation -- requires agents to send a lightweight status message type.

**8. Onboarding: first-minute experience (NEW -- elevated from polish to load-bearing)**

Edmondson's research shows that early interactions disproportionately determine whether a team environment feels safe or threatening. "The first few interactions matter disproportionately. If the first visible mistake in the log is met with blame, the channel is dead for learning." [Source: Edmondson deep read, Section 7]

Applied to bpmux: the first minute of using the tool sets the user's mental model. If the first experience is confusion about what commands exist, how to connect agents, or what the status bar means, the tool starts in a deficit.

Implementation:
- On launch, display "Resumed session abc123, 47 messages" vs "New session abc123" (v1 item #9)
- Keybinding help overlay (Ctrl+?) showing available commands (v1 item #8)
- Welcome message on first-ever launch explaining the conduit model

This is not polish. Edmondson's finding that "context support is positively associated with psychological safety" and that "access to resources and information is likely to reduce insecurity and defensiveness" means onboarding is infrastructure for the trust that makes the tool work. [Source: Edmondson deep read, Section 3]

**Confidence: high** that onboarding matters. **Medium** on the specific implementation -- what constitutes "enough" onboarding is unclear until we observe new users.

**9. Input line editing (readline basics)**

Cursor movement (left/right arrow), word-delete (Ctrl+W), line-clear (Ctrl+U). [Source: Nielsen's consistency heuristic; GNU readline conventions are deep muscle memory]

**Confidence: high** for necessity. **Medium** for timeline -- the designer flagged this as non-trivial in the terminal raw mode context.

**10. Message formatting (markdown-lite, code blocks only)**

Render code fences in dim/alternate style. Nothing else. The designer warned against the rabbit hole of full markdown parsing. The researcher called it "single highest-impact change" but the designer correctly flags the risk. [Source: v1 Tier 3 compromise]

Scoped to code block detection only. Validate that code blocks alone improve readability before expanding scope. This comes after truncation (item #2) -- truncation is prerequisite.

**Confidence: medium.** Impact is speculative until we see actual agent message patterns in the TUI.

### H1 Cut List

**Cut from v1 H1:** Nothing removed, but markdown rendering is explicitly scoped to code-blocks-only and gated behind message truncation. The ordering is strict: truncation ships before formatting.

### H1 Success Criteria

A developer uses bpmux as their daily AI collaboration tool without friction. They can:
- Tell agents apart visually (per-agent colors)
- Direct messages to specific agents (@-mention)
- Know when new messages arrive while in another pane (bell + unread indicator)
- See agent activity between messages (status indicator)
- Confirm agents understood instructions before acting (grounding convention)
- Read long agent messages without losing context (truncation)
- Discover commands without memorization (help overlay)
- Edit input without retyping (readline)

---

## Section 4: Horizon 2 -- Multi-Human Sessions

**Timeline:** 4-6 months after H1 (unchanged from v1).

**The honest framing:** H2 is the unvalidated bet. The researcher's critique is blunt: "No research supports mixed human-AI teams of 6-8 in a flat chat. The vision is building for a user that may not exist." No study has examined terminal-based multi-agent chat. All collaboration research assumes GUI environments with ample screen real estate. [Source: v1 Open Question #5; researcher critique; v1 Research Gap #7]

**Confidence: low** that multi-human sessions work as imagined. **High** that the minimum experiment (roster, tokens, snapshots) is worth running to find out.

### H2 Features (focused)

**1. Participant roster with role tags**

Named list of who is in the session and their role (driver, navigator, observer). Gutwin & Greenberg call a named participant list "the most basic awareness display." Currently bpmux shows "2 connected" but not who those 2 are. [Source: Gutwin & Greenberg deep read, Section 7, Gap 1]

With 2 humans + their agents (4-6 participants), identity legibility shifts from "nice to have" to "prerequisite for communication." Clark & Brennan's grounding techniques (alternative descriptions, trial references) all depend on knowing who you are grounding with. [Source: Clark & Brennan deep read, Section 3]

**2. Session join tokens**

Minimum access control for sharing sessions across machines/teams. A token that grants session access without requiring accounts or persistent identity.

This is per-session, ephemeral, and intentionally minimal. No user accounts. No persistent identity. No SSO. The contradiction from v1 is resolved by accepting that H2 identity is session-scoped and designing accordingly. If enterprise demand materializes in H3, persistent identity gets designed then. [Source: v1 Section 4, anti-feature contradiction]

**3. Async session snapshots**

A `/snapshot` command that generates a compressed summary of session state: what has been decided, what is in progress, what is next. This is the handoff mechanism for timezone-shifted collaboration.

Research backing: Mathieu et al. (2000) show teams with shared mental models perform better. A snapshot is an attempt to make the shared mental model explicit and transferable. The researcher correctly identifies that "a 500-message JSONL log is not a shared mental model." The snapshot is the bridge between raw log and usable context. [Source: Mathieu et al. 2000; v1 Tier 4 item #12]

**Confidence: medium.** The right format for snapshots is unclear. Manual vs. auto-generated vs. agent-maintained are all plausible. H1 dogfooding should reveal what "catching up" actually looks like.

**4. Driver/navigator indicator**

Show which participant holds the editing role. Transferable via command. With 2 humans, the social protocol for "who is editing" needs system support. [Source: mob programming research, Zuill 2014; v1 Tier 4 item #10]

**5. Grounding protocol (if convention from H1 proves insufficient)**

If the H1 convention-level grounding approach works, no protocol change needed. If it does not, H2 adds read cursors or structured acknowledgment messages. Clark & Brennan identify "continued attention" as the most basic form of positive evidence -- a "participant X has read up to message Y" cursor would provide this. [Source: Clark & Brennan deep read, Section 7]

### H2 Cut List (moved from v1 H2)

**Web UI -- moved to H3.** The designer critique is right: maintaining a web frontend alongside the Rust TUI is a significant context switch for one developer. The TUI is sufficient for active participants. Passive observers can wait. [Source: designer critique]

**Reply-to / thread references -- moved to H3.** The researcher correctly notes this is a "half-measure" that adds metadata without solving the scanning problem. Clark & Brennan's analysis suggests flat sequential chat with strong grounding conventions may outperform weak threading. Defer until real multi-human sessions reveal whether flat chat actually breaks at 4-6 participants. If it does, the solution may be message tiering (design principle #6) rather than reply-to. [Source: researcher critique; Clark & Brennan deep read, Section 4]

### The Hard Question (unchanged from v1)

The researcher says multi-human sessions are unvalidated. The designer says this is where the value proposition lives. The resolution: build the minimum (roster, tokens, snapshots), run real sessions, and let the data decide. If 2-human sessions work, double down. If they do not, H2 becomes "better 1-human sessions with more agents."

Carraro, Furlan & Netland (2025) confirm that shared mental models degrade with team size. Brooks (1975), validated by Chernoguz (2011), confirms communication overhead grows quadratically: 3 participants = 3 channels, 6 participants = 15 channels. A single shared channel collapses this architecturally but not cognitively. [Source: v1 Section 1]

---

## Section 5: Horizon 3 -- Platform

**Timeline:** 9+ months (unchanged from v1).

H3 is directional, not planned. Everything here depends on H1/H2 outcomes. Confidence levels reflect this.

### 5.1 A2A Protocol as Integration Path

**Confidence: medium** (upgraded from low in v1 based on market research).

v1 treated A2A as speculative ("Monitor, do not invest"). Market research changed this assessment. The protocol ecosystem has stabilized around a two-layer model:

- **MCP** (Model Context Protocol): vertical integration, agent-to-tool. 97M monthly SDK downloads as of Q1 2026. Adopted by every major AI provider. bpmux already uses MCP for its bridge. [Source: market research, protocol analysis]
- **A2A** (Agent-to-Agent Protocol): horizontal collaboration, agent-to-agent. Google donated to Linux Foundation under Apache 2.0 (April 2025, stabilized by December 2025). Handles agent discovery, delegation, and cross-tool collaboration. [Source: market research, A2A protocol findings]

bpmux currently requires each AI tool to connect via a bpmux-specific MCP bridge. A2A would let any A2A-compliant agent join a session through a standard protocol, eliminating the per-tool bridge requirement. This is the path from "works with Claude Code" to "works with any agent."

**What remains unvalidated:** Whether A2A's spec is stable enough to build against. Whether real tools implement it in production (vs. announcing support). Whether the overhead is justified for a local dev tool. The designer's original concern stands -- but the market evidence is stronger than v1 acknowledged. [Source: designer critique of v1]

**Decision point:** If by the time H2 ships, 3+ major AI coding tools support A2A in production, invest in A2A adapter. If not, continue with per-tool MCP bridges.

### 5.2 JSONL as Compliance Infrastructure

**Confidence: low for revenue, medium for positioning.**

The EU AI Act enters enforcement phase in 2026. High-risk agent systems require continuous audit trails, deterministic guardrails, and human oversight. Non-compliance penalties reach 30M EUR. [Source: market research, enterprise AI collaboration findings]

bpmux's append-only JSONL log captures exactly what compliance frameworks demand: who said what, when, what reasoning was exposed, what decisions were made. The log is already structured, timestamped, and attributable.

**What this means for H3:** The log format should be treated as a contract, not an implementation detail. Schema stability, export tooling, and integration with compliance platforms (e.g., Galileo, observability stacks) become features. The JSONL log is not just a session record -- it is audit infrastructure that enterprises need but currently build ad hoc.

**What remains unvalidated:** No enterprise buyer has been identified. The gap between "captures the right data" and "meets procurement requirements" is enormous. No auditor has accepted a JSONL log as compliance documentation. [Source: v1 Section 2]

### 5.3 Session Recordings as Knowledge Base

**Confidence: medium.**

Completed sessions represent externalized reasoning -- not just what was decided, but why. Clark & Brennan's reviewability property, taken to its logical conclusion, means past sessions are a searchable knowledge base of team decisions and AI reasoning traces.

Implementation path:
- JSONL index over completed sessions (search by participant, date, keyword)
- Summary generation from session logs (what was decided, what was built)
- Cross-session reference (link a current session to a past one for context)

**What would validate:** Users referencing past sessions to inform current decisions. Session logs being useful to someone who was not present. [Source: v1 Section 2; Gutwin & Greenberg past-tense awareness elements]

**What would invalidate:** Users never reading back through logs. Sessions being too noisy to extract signal from. The log's value being purely as agent context (fed back into AI) rather than as human-readable artifact.

### 5.4 Self-Hosted First (with adoption tradeoff acknowledged)

**Confidence: high** for the principle. **Medium** for the strategy.

The principle is sound: developer tooling culture favors local tools, AI reasoning traces are sensitive data, and self-hosted is the trust-maximizing deployment model. [Source: v1 Section 2]

The tradeoff the designer critique identified: "developer tools that require self-hosting have dramatically lower adoption than managed services." This is true. The market evidence for this is overwhelming (GitHub vs GitLab adoption curves, Slack vs Mattermost, etc.).

The resolution is not to abandon self-hosted but to acknowledge the cost. bpmux's H3 deployment model should be:
1. **Primary:** Single binary, runs locally, zero configuration. This is the current model and should remain the default.
2. **Secondary (if demand appears):** Managed service option for teams that want collaboration without ops burden. This is a business decision, not an architecture decision -- the current WebSocket architecture supports both models.

Do not pre-build managed infrastructure. Do acknowledge that self-hosted-only limits the addressable market.

### 5.5 Moved to H3 (from H2)

**Web UI (read-only observer).** Who specifically wants to watch a bpmux session passively? The answer to this question determines whether to build it. No evidence of demand yet. [Source: designer critique]

**Reply-to / thread references.** Depends on whether flat chat breaks at 4-6 participants (Open Question #5). If it does, this may be the right solution. If it does not, it is unnecessary complexity. [Source: researcher critique]

### 5.6 Speculative (unchanged from v1)

- **Federated sessions across networks.** Engineering fantasy with no demonstrated demand.
- **Organization-scoped sessions with SSO.** Requires enterprise buyers who do not exist yet. If they appear, persistent identity gets designed at that point -- not before.
- **Session templates.** One known workflow does not need templates.
- **Agent provider abstraction.** The bridge contract abstracted beyond Claude-specific MCP bridge. Depends on demand from users of other AI tools. A2A may subsume this.

---

## Section 6: Anti-Features (Confirmed, Updated)

### Confirmed by Both Research and Design

**No threading.** Flat chat preserves shared awareness for small teams (Gutwin & Greenberg). Threading fragments it. For 3 participants this is unambiguous. For 6+, re-evaluate based on actual H2 usage. [Source: Speakwise 2026 Slack threading research; v1 Section 6]

**No real-time collaborative editing.** bpmux is a communication channel, not a shared artifact editor. Mob programming research confirms the "single keyboard" constraint is a feature. [Source: Zuill 2014]

**No agent runtime.** Every framework (CrewAI, LangGraph, AutoGen) is an orchestrator. bpmux is infrastructure, not orchestration. Market research confirms this positioning is distinct and defensible. [Source: market research; v1 Section 6]

**No project management features.** The backlog lives in GitHub Issues. Workflow lives in process docs. [Source: v1 Section 6]

**No general-purpose chat features.** File sharing, voice/video, emoji reactions, app integrations serve "chat in general" and dilute the product. [Source: v1 Section 6]

**No mobile client.** The users are developers at workstations. [Source: v1 Section 6]

### Confirmed with Caveats (Updated)

**No persistent user accounts -- until enterprise demand proves real.** Per-session identity through H2. The v1 contradiction (reject accounts in H2, require them in H3) is resolved: identity is session-scoped and ephemeral until a paying customer needs otherwise. This is a conscious adoption tradeoff. [Source: v1 Section 4; designer critique]

**No multi-pane layouts -- status bar is the awareness surface.** The 80x24 constraint makes sidebars impractical. The status bar carries awareness information for H1. If H2 multi-human sessions need more, a minimal roster pane (5 characters wide) could work without sacrificing chat height. Decide based on H2 usage. [Source: Gutwin & Greenberg deep read, Section 5 on constrained displays; designer critique]

**No plugin/extension points -- but the bridge contract should be extensible.** The researcher correctly identifies that the H3 "agent adapter layer" is a plugin system by another name. Not building plugins is correct for now. Building a tight coupling that prevents future plugins would be wrong. [Source: researcher critique; v1 Section 6]

---

## Section 7: Open Questions (Updated)

### From Dogfooding (answerable now)

1. **What is the human-to-agent message ratio?** Instrument the log. Count it. Stop speculating.
2. **How long are agent messages in practice?** Same -- measure, then design truncation thresholds from data.
3. **Do users scroll back or stay at the bottom?** Track scroll position in the TUI. This determines whether unread indicators or catch-up UX is the higher priority.
4. **What triggers a human to check bpmux?** Observe our own behavior for a week. Proactive or reactive?

### From H2 Experiment (answerable only by running sessions)

5. **Does flat chat break at 4-6 participants?** The research predicts it might (Brooks, Chernoguz). The design insists it will not. Only real sessions settle this.
6. **Is the JSONL log actually useful for catch-up?** Do users read back, or just ask an agent "what happened?" If the latter, the log's value is as agent context, not human artifact.
7. **What does the grounding convention look like in practice?** Do agents naturally acknowledge, or do they skip the acceptance phase? Clark & Brennan predict they will skip it unless the convention is explicit. [Source: Clark & Brennan deep read, Section 7]

### Research Gaps That Remain

8. **No studies of terminal-based multi-agent chat.** All collaboration research assumes GUI environments.
9. **No data on AI message volume in shared channels.** Slack research measures human frequency (275 notifications/day). AI agents likely produce an order of magnitude more.
10. **No research on trust calibration across multiple AI agents in a coding context.** The SAT model addresses single-agent transparency. Multi-agent trust calibration is unstudied.
11. **No established patterns for grounding in human-AI text chat.** Clark & Brennan's framework predates AI. What "acceptance" means when one party is an LLM is an open question. [Source: Clark & Brennan deep read, Section 7]

---

## Section 8: Confidence Summary

| Claim | Confidence | Source | Status |
|---|---|---|---|
| Persistent log is the core primitive | High | Clark & Brennan (reviewability), Gutwin & Greenberg (past-tense elements), cognitive/intent debt paper | Validated by architecture |
| Grounding mechanism is H1 priority | High | Clark & Brennan (contribution model, acceptance phase) | Validated by research, untested in implementation |
| 3-participant sweet spot works | High | Mathieu et al. (shared mental models), Brooks/Chernoguz (communication overhead) | Validated by dogfooding |
| Message truncation before rendering | High | Dogfooding observation | Validated by daily use |
| Onboarding is load-bearing | High | Edmondson (early interactions set safety trajectory) | Validated by research, untested in bpmux |
| A2A is the integration path | Medium | Market research (protocol ecosystem stabilization, Linux Foundation governance) | Partially validated by market data |
| JSONL as compliance infrastructure | Medium | EU AI Act 2026 enforcement, market research | Speculative -- no enterprise buyer identified |
| Multi-human sessions work | Low | No supporting research; Carraro et al. predicts degradation | Entirely unvalidated |
| Session recordings as knowledge base | Medium | Clark & Brennan (reviewability), Gutwin & Greenberg (event history) | Plausible but unvalidated by usage |
| Self-hosted limits adoption | High | Market precedent (GitHub vs GitLab, Slack vs Mattermost) | Validated by industry data |
| Flat chat survives 6 participants | Low | Brooks predicts it won't; Speakwise data is ambiguous for AI agents | Must test in H2 |

---

## Appendix: Source Index

### Primary Research (Deep Reads Completed)

- Clark, H.H. & Brennan, S.A. (1991). "Grounding in Communication." In *Perspectives on Socially Shared Cognition*. APA Books, Ch. 7, pp. 127-149. [Deep read: `/docs/research/foundations/clark-brennan-deep-read.md`]
- Gutwin, C. & Greenberg, S. (2002). "A Descriptive Framework of Workspace Awareness for Real-Time Groupware." *CSCW*, 11, 411-446. [Deep read: `/docs/research/foundations/gutwin-greenberg-deep-read.md`]
- Edmondson, A.C. (1999). "Psychological Safety and Learning Behavior in Work Teams." *Administrative Science Quarterly*, 44(2), 350-383. [Deep read: `/docs/research/foundations/edmondson-deep-read.md`]

### Academic / Secondary Research

- Mathieu, J.E. et al. (2000). "The Influence of Shared Mental Models on Team Process and Performance." *Journal of Applied Psychology*, 85(2), 273-283.
- Carraro, Furlan & Netland (2025). "Shared Mental Models and Proactive Problem-Solving."
- Chen, J.Y.C. et al. (2014). Situation Awareness-based Agent Transparency (SAT) model.
- Endsley, M.R. (2017). Trust calibration in agent transparency.
- Lou et al. (2025). "Unraveling Human-AI Teaming." Situational trust findings.
- Sacks, H., Schegloff, E.A., & Jefferson, G. (1974). "A Simplest Systematics for the Organization of Turn-Taking for Conversation."
- Murder Mystery Agents (Frontiers in AI, 2025). Multi-party AI turn-taking with adjacency pairs.
- Zuill, W. (2014). "Mob Programming." Agile Alliance.
- Brooks, F.P. (1975). *The Mythical Man-Month*. Validated by Chernoguz (2011) across 7,200 projects.
- Nielsen, J. (1994). "10 Usability Heuristics for User Interface Design."
- Case, A. (2015). *Calm Technology*.
- Tufte, E.R. (1983). *The Visual Display of Quantitative Information*.
- Cognitive/Intent Debt paper (arxiv, March 2026). Fowler fragment (April 2, 2026).

### Industry / Market Research

- ChatCollab: Human-AI Collaborative Software Teams (Klieger et al., 2024).
- GitHub Developer Survey 2025 (17% collaboration satisfaction metric).
- Slack Messaging Statistics (Speakwise, 2026).
- A2A Protocol -- Google/Linux Foundation, Apache 2.0 (April 2025, donated December 2025).
- MCP adoption data (97M monthly SDK downloads, Q1 2026).
- EU AI Act enforcement timeline (2026, penalties up to 30M EUR).
- Deloitte AI Agent Market Projections ($8.5B by 2026).
- Cursor/Windsurf/Claude Code market analysis (various, 2026).

### Critique Sources

- Researcher critique of design vision (Wave 2).
- Designer critique of research (Wave 2).
- Market research findings (Wave 3).
