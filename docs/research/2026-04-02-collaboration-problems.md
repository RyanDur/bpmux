# Collaboration Problems That Remain After AI Solved "I Don't Know How to Code This"

**Date:** 2026-04-02

AI coding assistants have largely eliminated the "blank page" problem. Developers can generate working implementations for most tasks in minutes. But code production was never the bottleneck. The hard problems in software development have always been coordination problems, and AI has made several of them worse.

---

## 1. Architectural Alignment

**The problem:** AI can propose different architectures to different developers on the same team. Developer A asks Claude for a caching layer and gets an event-driven design. Developer B asks for the same feature and gets a request-response pattern. Both implementations work in isolation. Neither works together.

**What the research shows:** Teams that lean heavily on AI generation without upfront alignment produce what one developer described as "duplicate logic, mismatched method names, no coherent architecture" (Addy Osmani, 2026). Multi-agent systems attempt to solve this with orchestrator patterns that coordinate specialized agents, but this pushes architectural decisions into tooling rather than team understanding.

**The gap:** There is no mechanism for AI assistants to share context across developers. Each session starts fresh. The architectural vision that used to live in shared conversations, whiteboard sessions, and pairing now lives nowhere.

**Sources:**
- [Addy Osmani - My LLM coding workflow going into 2026](https://medium.com/@addyosmani/my-llm-coding-workflow-going-into-2026-52fe1681325e)
- [The State of AI Coding Agents (2026)](https://medium.com/@dave-patten/the-state-of-ai-coding-agents-2026-from-pair-programming-to-autonomous-ai-teams-b11f2b39232a)

---

## 2. Shared Understanding of the Codebase (Cognitive Debt)

**The problem:** When two developers change the same system with AI help, neither may understand the other's changes. AI generates code faster than humans can build mental models of it. The output is syntactically clean and well-formatted, which historically triggered merge confidence, but the team's collective understanding erodes.

**What the research shows:** Margaret-Anne Storey and colleagues (2026) define **cognitive debt** as "the erosion of shared understanding among developers about what a system does, why design decisions were made, and how it can be changed." Unlike technical debt, which resides in code, cognitive debt lives in developers' minds. It accumulates when teams prioritize speed over comprehension. The warning signs: hesitation about making changes, over-reliance on individual expertise, and the system becoming a "black box."

A companion concept, **intent debt**, describes "the absence of externalized rationale that both developers and AI agents need to work safely with code" (Storey et al., arXiv 2603.22106). The two debts interact: without shared understanding (cognitive) and without documented reasoning (intent), teams cannot effectively review or guide machine-generated output.

**The gap:** Traditional practices that built shared understanding (pairing, mob programming, code review discussions) are being displaced by solo-plus-AI workflows. The feedback loop that turned code review into a learning conversation is breaking down when the code's author cannot explain the AI's implementation choices.

**Sources:**
- [Margaret-Anne Storey - How Generative and Agentic AI Shift Concern from Technical Debt to Cognitive Debt (2026)](https://margaretstorey.com/blog/2026/02/09/cognitive-debt/)
- [Storey et al. - From Technical Debt to Cognitive and Intent Debt (arXiv 2603.22106)](https://arxiv.org/abs/2603.22106)
- [Addy Osmani - Comprehension Debt: The Hidden Cost of AI-Generated Code (2026)](https://addyosmani.com/blog/comprehension-debt/)
- [Matt Hopkins - Cognitive Debt: The Hidden Cost of Letting AI Write Your Code](https://matthopkins.com/technology/cognitive-debt-the-hidden-cost-of-letting-ai-write-your-code/)

---

## 3. Design Decisions and Their Rationale

**The problem:** AI generates code fast, but the "why" behind design choices is lost. Decisions that used to emerge from team discussion and get captured in meeting notes, Slack threads, or institutional memory now happen in ephemeral AI chat sessions that disappear when the window closes.

**What the research shows:** Architectural Decision Records (ADRs) have been a recommended practice since Michael Nygard's original proposal, and Martin Fowler's documentation of the pattern remains a reference point. ADRs capture the context, decision, and consequences of architectural choices. The practice is more important than ever: when AI can generate a working implementation in minutes, the decision of *which* implementation to choose and *why* needs to be preserved for the team.

Emerging tools like Kawa attempt to address this by tracking AI reasoning and detecting conflicts across sessions. The approach of having AI generate ADR drafts from one-line decision summaries shows promise for reducing the documentation burden while preserving rationale.

**The gap:** Most AI coding workflows have no persistence layer for design reasoning. The conversation where the developer and AI explored three approaches and chose one is gone after the session ends. The next developer (or the same developer next month) sees only the final code with no trace of the alternatives considered or trade-offs evaluated.

**Sources:**
- [Martin Fowler - Architecture Decision Record](https://martinfowler.com/bliki/ArchitectureDecisionRecord.html)
- [ADR GitHub Organization](https://adr.github.io/)
- [Tom Gerken - ADR for use with AI coding tools](https://github.com/tom-gerken/ADR)
- [Equal Experts - Accelerating ADRs with Generative AI](https://www.equalexperts.com/blog/our-thinking/accelerating-architectural-decision-records-adrs-with-generative-ai/)
- [Kawa: AI Coding Assistant Context, Intent & Decision Tracking](https://mcpmarket.com/server/kawa)

---

## 4. Conflicting Changes at AI Velocity

**The problem:** When AI helps both developers work faster, the volume of changes increases but the coordination mechanisms remain human-speed. More code produced per unit time means more surface area for conflicts, and the conflicts are harder to resolve because neither developer may fully understand the AI-generated code they are merging.

**What the research shows:** This is an amplification of Brooks' Law: adding manpower to a late project makes it later, because communication overhead grows quadratically with team size. AI does not add people, but it multiplies each person's output, creating the same effect. Two developers producing at 5x velocity create the coordination burden of a ten-person team without any of the communication structures that a ten-person team would build.

AI-assisted coding introduces version control challenges around tracking who (or what) made specific code changes. When an AI generates a 200-line refactor and a human generates a 200-line refactor of overlapping code, the merge conflict is between two parties who may not understand their own changes well enough to resolve the conflict correctly.

**The gap:** Current tooling treats AI-generated code identically to human-written code in version control. There is no mechanism to flag that a change was AI-generated, capture the intent behind it, or provide the AI's reasoning during conflict resolution. Merge tools optimize for textual conflicts, not semantic ones.

---

## 5. Trust and Accountability

**The problem:** Who is responsible for AI-generated code? When a bug ships in code that an AI wrote but a human approved, the accountability is unclear. This ambiguity affects team dynamics: developers may rubber-stamp AI output they do not fully understand, or conversely, spend excessive time reviewing code they could have written faster themselves.

**What the research shows:** Research on trust in AI code generation tools (ACM FAccT 2024) finds that developer trust depends on the AI tool's perceived ability, integrity, and benevolence, and is highly situational. Teams that treat LLMs like assistants but fail to assign ownership for what they create allow low-quality code to slip through. The emerging consensus is that accountability belongs to the person who approves the code, not the tool that generated it, but this requires reviewers to actually understand what they are approving.

New governance structures are forming around this: requiring that at least one human on the team fully understands each AI-generated change before it ships, treating AI output as a draft that requires the same scrutiny as a junior developer's PR, and establishing clear policies about AI tool usage in the development lifecycle.

**The gap:** Most teams have no explicit policy on AI-generated code accountability. The social contract of code ownership ("you wrote it, you own it") breaks down when a developer can say "the AI wrote that part." Teams need new norms that assign responsibility for AI-assisted output.

**Sources:**
- [Investigating and Designing for Trust in AI-powered Code Generation Tools (ACM FAccT 2024)](https://dl.acm.org/doi/abs/10.1145/3630106.3658984)
- [The New Rules of AI-Generated Code Accountability](https://asymm.com/the-new-rules-of-ai-generated-code-accountability/)
- [Trustworthy AI Software Engineers (arXiv 2602.06310)](https://arxiv.org/html/2602.06310v1)

---

## 6. The Coordination Problem at Scale

**The problem:** AI makes individuals faster but does not reduce coordination cost. A team of five developers each working at 5x velocity does not ship 25x faster. They ship at whatever rate the coordination bottleneck allows, and that bottleneck is now more severe because the volume of changes requiring coordination has increased.

**What the research shows:** Brooks observed in 1975 that the number of communication channels grows as n(n-1)/2 for a team of n people. AI does not change this formula. It changes the throughput of each node while leaving the graph topology identical. Tasks that once required cross-team coordination can become focused working sessions for an individual, but the integration points between those sessions remain human-speed bottlenecks.

Multi-agent architectures attempt to address this by having AI systems coordinate with each other, but this trades one coordination problem (human-human) for another (human-agent-agent-human) with less transparency and fewer established practices.

**The gap:** The industry has optimized for individual developer productivity (Copilot, Claude Code, Cursor) without proportional investment in team coordination tooling. Code review, architecture discussion, and integration testing remain largely manual, human-speed processes operating on a firehose of AI-accelerated output.

---

## 7. Cultural and Social Aspects

**The problem:** Pair programming built relationships, shared culture, and distributed knowledge. When developers work solo with AI assistants instead of human colleagues, they stop sharing knowledge about technical aspects and business requirements within the team. The social fabric of the team weakens.

**What the research shows:** Research comparing human-human pairs, human-AI pairs, and solo-plus-AI developers found that the highest performance came from human+AI pairs working together, outperforming both two humans together and a single developer using AI alone (Friedrich Kurz, 2025). This suggests the social and cognitive benefits of pairing are not redundant with AI assistance; they are complementary.

Stack Overflow's analysis (2024) argues that developers using AI assistants should follow the pair programming model, treating the AI as a navigator while maintaining the discipline of the pairing practice. However, this framing misses the team-level benefits: pair programming was never just about the code produced in a single session. It was about the relationships, shared vocabulary, and distributed understanding that emerged across many sessions.

Teams that were already lukewarm on pair programming have largely let it fade in favor of solo-plus-AI workflows. The knowledge sharing, mentorship, and cultural transmission that happened during pairing sessions now has no replacement.

**The gap:** AI replaces the technical half of pair programming (a second perspective on the code) but not the social half (relationship building, culture transmission, mentorship, shared ownership). Teams need new practices to serve these social functions.

**Sources:**
- [Friedrich Kurz - Pair Programming in the Age of AI (2025)](https://friedrichkurz.me/posts/2025-03-25-pair-programming-in-the-age-of-ai/)
- [Stack Overflow - Developers with AI assistants need to follow the pair programming model (2024)](https://stackoverflow.blog/2024/04/03/developers-with-ai-assistants-need-to-follow-the-pair-programming-model/)
- [Pair Programming & TDD in 2025: Evolving or Obsolete in an AI-First Era](https://medium.com/@pravir.raghu/pair-programming-tdd-in-2025-evolving-or-obsolete-in-an-ai-first-era-00680ce93695)

---

## Synthesis

The common thread across all seven problems is **shared understanding**. AI solved the production problem (generating code) but amplified the coordination problem (ensuring the team collectively understands and agrees on what is being built and why).

The problems cluster into three categories:

1. **Knowledge problems** (1, 2, 3): The team does not share a common understanding of the architecture, the codebase, or the reasoning behind decisions.
2. **Process problems** (4, 5, 6): The mechanisms for coordinating work (version control, code review, accountability) were designed for human-speed development and break under AI-accelerated output.
3. **Social problems** (7): The practices that built team cohesion and distributed knowledge are being displaced without replacement.

A tool that addresses these problems would need to make AI reasoning visible across the team, persist design rationale beyond individual sessions, and create new surfaces for the shared understanding that used to emerge from human-to-human collaboration.
