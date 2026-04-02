# Supporting Research for bpmux

*Compiled April 1, 2026. Academic and industry research that reinforces bpmux's design approach.*

**Unifying lens:** bpmux is a stigmergic coordination system — a shared environment where traces left by one participant stimulate action by another. Each research domain below validates a different facet of this approach: shared mental models explain *why* the shared environment builds understanding, transactive memory explains *how* "who knows what" becomes visible, workspace awareness explains *what* participants need to see, context switching costs explain *what happens without* a shared environment, and the efficiency paradox explains *why there's a market* for one.

---

## 1. Pair Programming Effectiveness

**Key finding:** Pair programming produces higher quality code on complex tasks, at the cost of greater effort.

The definitive meta-analysis (Hannay et al., 2009 — 18 studies) found that pair programming has a small positive effect on code quality, a medium positive effect on completion time, and a medium negative effect on effort (it takes more total person-hours). Critically, **task complexity is the moderating variable**: pairing is most valuable for complex tasks where quality matters, and least valuable for simple tasks where speed matters.

**Why this reinforces bpmux:** bpmux targets the scenario where pairing is most effective — complex tasks on real client engagements. The "effort cost" of pairing (two people working on one thing) is exactly what bpmux aims to reduce by making the AI carry some of the cognitive load. If each developer's AI handles the grunt work while the humans pair on architecture and decisions, you get the quality benefits of pairing without the full effort penalty.

**Source:** [The effectiveness of pair programming: A meta-analysis](https://www.sciencedirect.com/science/article/abs/pii/S0950584909000123) (Information and Software Technology, 2009)

---

## 2. Context Switching Costs

**Key finding:** It takes 23 minutes to regain focus after an interruption. Interrupted tasks take twice as long and contain twice as many errors.

Gloria Mark's research at UC Irvine found that knowledge workers switch tasks every 3 minutes on average and experience significant interruptions every 11 minutes. Each switch costs cognitive residue — attention that stays stuck on the previous task for 30-60 minutes (Sophie Leroy, University of Washington). A single unplanned context switch consumes up to 20% of cognitive capacity.

**Why this reinforces bpmux:** The current "summarize context, paste into commit message, partner's AI reads it" workflow is a forced context switch — not just for the human, but for the AI. The AI loses its entire mental model and rebuilds from a summary. bpmux eliminates this switch by keeping both AIs in the loop continuously. No context is lost, no rebuild required. The persistent message log means even a new AI session can pick up without the 23-minute recovery.

**Sources:**
- [Gloria Mark, UC Irvine — Context switching research](https://www.ics.uci.edu/~gmark/)
- [Sophie Leroy — Attention residue](https://www.sciencedirect.com/science/article/abs/pii/S0749597809000399)
- [Context Switching Cost for Developers](https://super-productivity.com/blog/context-switching-costs-for-developers/)

---

## 3. Shared Mental Models

**Key finding:** Teams with shared mental models coordinate more effectively, especially when distributed.

Espinosa et al. studied distributed software teams and found that shared mental models — common understanding of the task, the codebase, and who knows what — directly improve coordination and reduce development time. The effect is **stronger for geographically distributed teams** than co-located ones, because distributed teams can't rely on ad-hoc conversations to stay aligned.

**Why this reinforces bpmux:** When two developers pair with AI assistants, there are four participants who need a shared mental model. Without bpmux, the AIs are blind to each other — they have separate, potentially conflicting models of the codebase. bpmux creates a shared information space where all four participants maintain a common understanding. The AI-to-AI communication isn't just convenient; it's building shared mental models across the entire team (humans + AIs).

**Source:** [Shared Mental Models and Coordination in Large-Scale, Distributed Software Development](https://www.semanticscholar.org/paper/Shared-Mental-Models-and-Coordination-in-Software-Espinosa-Kraut/47fe3d078092e91288a38c2722e6ffdc8755dd1c)

---

## 4. Transactive Memory Systems

**Key finding:** Teams perform better when members know who knows what, and can access each other's expertise.

Transactive memory (Wegner, 1987) is a team's meta-knowledge: not just what the team knows collectively, but who knows which part. High-performing teams have three properties: **specialization** (each member is expert in something different), **credibility** (members trust each other's expertise), and **coordination** (members know who to ask for what). Information technology supports transactive memory by making expertise visible and accessible.

**Why this reinforces bpmux:** bpmux's sender identity system (`ryan:claude`, `alice:opencode`) and the transparency principle ("everything is visible to everyone") build transactive memory across the human-AI team. When you see `alice:claude` reasoning about the auth module, you know that Alice's AI has expertise there. The trust levels (local, local-ai, peer, peer-ai) map directly to the credibility dimension. The visibility markers show who has seen what context — making the "who knows what" explicit and visible.

**Sources:**
- [Transactive Memory Systems — Wegner, 1987](https://en.wikipedia.org/wiki/Transactive_memory)
- [The Impact of IT and TMS on Knowledge Sharing and Team Performance](https://misq.umn.edu/the-impact-of-information-technology-and-transactive-memory-systems-on-knowledge-sharing-application-and-team-performance-a-field-study.html)

---

## 5. Workspace Awareness (CSCW)

**Key finding:** Real-time awareness of collaborators' activities is critical for fluid coordination.

Gutwin and Greenberg's framework for workspace awareness (2002) identifies the elements that collaborators need to know: who is in the workspace, where they are working, what they are doing, what they intend to do next. Shared feedback through a common workspace allows group members to "flexibly and fluidly coordinate their work" without explicit communication.

**Why this reinforces bpmux:** bpmux's chat is a shared workspace with real-time awareness. The status bar shows who is connected, the visibility markers show who has seen what, the inactivity detection shows who has gone silent. The TUI/Web UI provides the "shared feedback" that Gutwin and Greenberg identify as the key enabler of fluid coordination. The AI-to-AI messages are a form of workspace awareness that doesn't exist in any other tool — you can see what your partner's AI is thinking about.

**Sources:**
- [A Descriptive Framework of Workspace Awareness for Real-Time Groupware](https://link.springer.com/article/10.1023/A:1021271517844)
- [Awareness and Coordination in Shared Workspaces — Dourish & Bellotti](https://www.semanticscholar.org/paper/Awareness-and-coordination-in-shared-workspaces-Dourish-Bellotti/fc0ccc4e5c0f1f1f1eef1bc9a5a840d1a8e2fd95)

---

## 6. Human-AI Complementarity

**Key finding:** Human-AI teams outperform either alone when roles leverage complementary strengths — but this is rarely achieved in practice.

Research on complementarity in human-AI collaboration (2024-2025) shows that the ideal is "complementary team performance — a level of performance that neither can attain individually." However, this is **rarely observed** in practice, suggesting insufficient understanding of how to design for it. The key: AI automates easy tasks, augments humans on medium tasks, and humans handle hard tasks without AI. Optimized role specification leads to faster decisions, higher quality, and greater satisfaction.

**Why this reinforces bpmux:** The Ground Control / Major Tom working agreement is a role specification design: Claude handles code generation (where AI excels), the human handles decisions, architecture, and quality judgment (where humans excel). bpmux makes this complementarity work across a pair by keeping both humans and both AIs in their optimal roles simultaneously. The transparency principle ensures the human can always override the AI — which the research identifies as critical for achieving complementary performance.

**Source:** [Complementarity in Human-AI Collaboration: Concept, Sources, and Evidence](https://www.tandfonline.com/doi/full/10.1080/0960085X.2025.2475962)

---

## 7. Verification as the Bottleneck

**Key finding:** In multi-agent AI coding, generation is no longer the bottleneck — verification is.

Addy Osmani's analysis of multi-agent coding patterns (2026) identified that agents produce code faster than it can be verified. The most reliable systems gate transitions (plan → implement → review) rather than just outputs. Quality hooks at each transition prevent low-quality work from compounding.

**Why this reinforces bpmux:** The TDD-first, outside-in development approach with human sign-off at every step is exactly the gating pattern that works. The working agreement (Opus architects, Sonnet implements, Haiku verifies, Ground Control decides) creates four verification gates per cycle. bpmux's transparency principle means nothing is verified in secret — the human sees all AI reasoning and all AI-to-AI communication.

**Source:** [The Code Agent Orchestra — Addy Osmani](https://addyosmani.com/blog/code-agent-orchestra/)

---

## Summary: How bpmux Maps to Research

| Research Domain | Key Finding | bpmux Design Element |
|---|---|---|
| Pair programming | Quality improves on complex tasks | Targets complex engagements; AI reduces effort penalty |
| Context switching | 23 min recovery; 2x errors on interrupted tasks | Persistent log eliminates rebuild; read cursor survives restarts |
| Shared mental models | Shared understanding improves coordination (especially distributed) | All 4 participants share one conversation; AI-to-AI builds shared models |
| Transactive memory | "Who knows what" drives team performance | Sender identity, visibility markers, trust levels make expertise visible |
| Workspace awareness | Real-time awareness enables fluid coordination | Status indicators, chat stream, inactivity detection |
| Human-AI complementarity | Role specialization + human override = best performance | Ground Control owns decisions; AI writes code; transparency ensures override |
| Verification bottleneck | Gate transitions, not just outputs | TDD, human sign-off, multi-model review chain |
