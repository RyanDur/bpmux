# bpmux: Collective Memory for Human-AI Teams

*The founding thesis, v2. April 4, 2026.*

---

## The Problem

AI coding tools solved the wrong bottleneck. Cursor, Copilot, and Claude Code make individual developers faster at producing code. But code production was never the hard part of software development. Coordination was. And AI has made coordination worse.

When every developer on a team works at 5x velocity with their own AI, the volume of changes requiring coordination increases while the mechanisms for coordination remain human-speed (Brooks, 1975). Two developers producing at 5x velocity create the coordination burden of a ten-person team without any of the communication structures a ten-person team would build. The industry optimized for individual throughput and left coordination to rot.

The result is seven problems that AI created or amplified:

1. **Architectural alignment** -- AI proposes different architectures to different developers on the same team. Each session starts fresh. The shared vision that used to live in whiteboard sessions and pairing now lives nowhere (Osmani, 2026).
2. **Cognitive debt** -- AI generates code faster than teams can build mental models of it. Shared understanding erodes. Team members can no longer articulate why design decisions were made or how components interact (Storey et al., 2026).
3. **Lost rationale** -- Decisions that used to emerge from team discussion now happen in ephemeral AI sessions that disappear when the window closes. The code ships; the reasoning evaporates (Nygard; Fowler).
4. **Conflicts at AI velocity** -- More code per unit time means more surface area for conflicts, and the conflicts are harder to resolve because neither developer may fully understand the AI-generated code they are merging.
5. **Trust and accountability** -- When a bug ships in code an AI wrote but a human approved, accountability is unclear. Developers rubber-stamp output they do not fully understand (ACM FAccT, 2024).
6. **Coordination at scale** -- AI makes individuals faster but does not reduce coordination cost. The industry has invested in individual productivity without proportional investment in team coordination tooling.
7. **Social fabric** -- Pair programming built relationships, shared culture, and distributed knowledge. Solo-plus-AI workflows have displaced it. The social half of pairing -- mentorship, culture transmission, shared ownership -- has no replacement (Kurz, 2025).

These problems share a root cause. Communication is not transmission -- it is the joint establishment of mutual belief (Clark & Brennan, 1991). AI tools treat communication as transmission: send a message, assume it lands. But shared understanding requires both a presentation and an acceptance -- evidence that the recipient understood what the sender meant. Without that acceptance phase, messages are broadcast but never grounded. The team has information but not understanding.

## What bpmux Is

bpmux is collective memory for human-AI teams. It is a persistent, transparent, shared message bus where every participant -- human and AI -- can see the reasoning behind the work. The log captures not just what was decided but why: the tradeoffs weighed, the alternatives rejected, the assumptions stated.

The persistent log is the primitive. It prevents intent debt (Storey et al., 2026) by capturing rationale as a side effect of normal conversation. It prevents cognitive debt by making AI reasoning visible across the team rather than locked in individual sessions. It creates the shared context that coordination requires.

But the log alone is not enough. A log captures everything; participants need compressed, live state representation -- who is here, what are they doing, where are they focused (Gutwin & Greenberg, 2002). This is the real-time manifestation of cognitive debt: even when the reasoning exists in the record, teams cannot coordinate if they cannot see the current state of the collaboration at a glance. bpmux must surface workspace awareness -- presence, activity, intention -- alongside the persistent record.

## Where bpmux Differentiates

Cognitive debt is the theoretical problem. Coordination is the practical mechanism. Social fabric is the emergent outcome. All three matter. bpmux differentiates on coordination.

Every AI coding tool is productivity-first: make the individual faster. bpmux is coordination-first: make the team smarter. The difference is architectural. Productivity tools optimize the node; bpmux optimizes the graph. A shared channel where AI reasoning is visible, persistent, and reviewable gives teams something they have never had: access to the full deliberation behind every decision, not just the final answer.

This is not a feature of existing tools. Cursor does not know what Copilot is doing. Claude Code sessions are stateless and solitary. The AIs are individually competent and collectively blind. bpmux connects them through a shared communication medium that supports the three mechanisms Gutwin and Greenberg identified for gathering awareness: consequential communication (seeing activity), feedthrough (seeing artifacts change), and intentional communication (explicit messages). The persistent log adds what no physical workspace provides: perfect reviewability, so participants who join late can reconstruct the full reasoning history.

## What bpmux Requires

### Psychological safety is a prerequisite

bpmux requires psychological safety to function. Transparency without trust is surveillance.

Edmondson (1999) found that the relationship between structural features like transparent information sharing and team performance is fully mediated by psychological safety. Context support -- shared reasoning, persistent history, transparent AI output -- is positively associated with safety, but insufficient without it. The Publications Team in her study had adequate context support and no safety. The Stain Team had inadequate support but high safety. Safety is what determines whether the tool accelerates learning or amplifies threat.

The relationship is co-developmental. bpmux can help build safety -- access to information reduces insecurity and defensiveness (Edmondson, 1999, p. 375). But it can destroy safety if first interactions go wrong. The first visible mistake in the log met with blame makes the channel dead for learning. The first mistake met with curiosity makes it safe. Once established, the pattern self-seals in either direction: safe teams share more, which builds more safety; unsafe teams share less, which confirms the threat.

bpmux must not make things worse. No features that enable evaluative use of the log. No scoring, rating, or highlighting errors. The log is a shared workspace for reasoning, not a performance record.

### AI agents must be made to ground

AI agents skip the acceptance phase of communication (Clark & Brennan, 1991). They read the log, process it, and act on it without signaling comprehension. They do not ask clarifying questions unless prompted. They do not acknowledge receipt unless instructed. They assume understanding rather than verifying it.

This is not a behavioral quirk -- it is a structural property of the medium. Clark and Brennan's framework shows that grounding requires positive evidence of understanding: acknowledgments, relevant next turns, continued attention. AI agents provide none of these voluntarily. bpmux must make agents acknowledge understanding structurally -- through explicit addressing conventions, acceptance signals, and a norm that silence is ambiguous, never confirmation (Clark & Brennan, 1991; design principle 3).

Without this, the log becomes a broadcast medium rather than a grounding medium. Messages are transmitted but never established as mutual belief. The cognitive debt that bpmux exists to prevent accumulates silently because the acceptance phase never happens.

## The Thesis

bpmux exists because AI made teams faster at producing code and worse at understanding it. The seven problems above are symptoms of a single failure: the displacement of coordination practices (pairing, code review, architecture discussions) by solo-plus-AI workflows that optimize individual output at the expense of shared understanding.

bpmux is the coordination layer. The persistent log captures reasoning. Workspace awareness surfaces live state. The shared channel makes AI deliberation visible. Psychological safety determines whether any of it works. Structural grounding ensures that AI participants do not just broadcast but actually establish mutual understanding.

The persistent log is the primitive. Everything else is built on top of it.

---

## Sources

- Brooks, F. P. (1975). *The Mythical Man-Month*. Addison-Wesley.
- Clark, H. H. & Brennan, S. A. (1991). Grounding in Communication. In *Perspectives on Socially Shared Cognition*. APA Books.
- Edmondson, A. (1999). Psychological Safety and Learning Behavior in Work Teams. *Administrative Science Quarterly*, 44(2), 350-383.
- Gutwin, C. & Greenberg, S. (2002). A Descriptive Framework of Workspace Awareness for Real-Time Groupware. *Computer Supported Cooperative Work*, 11, 411-446.
- Kurz, F. (2025). Pair Programming in the Age of AI. friedrichkurz.me.
- Osmani, A. (2026). My LLM Coding Workflow Going Into 2026. medium.com.
- Storey, M.-A. et al. (2026). From Technical Debt to Cognitive and Intent Debt. arXiv:2603.22106.
- Investigating and Designing for Trust in AI-powered Code Generation Tools. ACM FAccT (2024).
