# Supporting Research for bpmux

*Compiled April 1, 2026. Academic and industry research that reinforces bpmux's design approach.*

**Unifying lens:** bpmux is a stigmergic coordination system — a shared environment where traces left by one participant stimulate action by another. Each research domain below validates a different facet of this approach: shared mental models explain *why* the shared environment builds understanding, transactive memory explains *how* "who knows what" becomes visible, workspace awareness explains *what* participants need to see, context switching costs explain *what happens without* a shared environment, and the efficiency paradox explains *why there's a market* for one.

---

## 1. Pair Programming Effectiveness

**Key finding:** Pair programming produces higher quality code on complex tasks, at the cost of greater effort.

The definitive meta-analysis (Hannay et al., 2009 — 18 studies) found that pair programming has a small positive effect on code quality, a medium positive effect on completion time, and a medium negative effect on effort (it takes more total person-hours). Critically, **task complexity is the moderating variable**: pairing is most valuable for complex tasks where quality matters, and least valuable for simple tasks where speed matters.

**Why this is relevant to bpmux:** bpmux targets the scenario where pairing is most effective — complex tasks on real client engagements. Our hypothesis: the "effort cost" of pairing (two people working on one thing) could be reduced by having AI carry some of the cognitive load. Whether this actually works is untested — the research validates the pairing context, not bpmux's specific approach to improving it.

**Source:** [The effectiveness of pair programming: A meta-analysis](https://www.sciencedirect.com/science/article/abs/pii/S0950584909000123) (Information and Software Technology, 2009)

---

## 2. Context Switching Costs

**Key finding:** It takes 23 minutes to regain focus after an interruption. Interrupted tasks take twice as long and contain twice as many errors.

Gloria Mark's research at UC Irvine found that knowledge workers switch tasks every 3 minutes on average and experience significant interruptions every 11 minutes, taking an average of 23 minutes and 15 seconds to fully regain focus. Each switch costs attention residue — attention that stays stuck on the previous task (Sophie Leroy, University of Washington). Rubinstein, Meyer & Evans (2001, *Journal of Experimental Psychology*) demonstrated increasing time costs with task complexity. Meyer estimated in public commentary that task switching can cost as much as 40% of productive time — this figure is from his interviews, not directly from the paper's experimental data.

**Why this is relevant to bpmux:** The current "summarize context, paste into commit message, partner's AI reads it" workflow is a forced context reconstruction. bpmux's hypothesis: a persistent shared message log reduces this cost by providing continuity across AI sessions. The context switching research validates that the *problem* is real and costly. Whether bpmux's approach to solving it is effective remains to be tested through dogfooding.

**Sources:**
- [Gloria Mark, UC Irvine — Context switching research](https://www.ics.uci.edu/~gmark/)
- [Sophie Leroy — Attention residue](https://www.sciencedirect.com/science/article/abs/pii/S0749597809000399)
- [Rubinstein, Meyer & Evans (2001) — Executive Control of Cognitive Processes in Task Switching (APA)](https://www.apa.org/pubs/journals/releases/xhp274763.pdf)
- [APA — Multitasking: Switching Costs](https://www.apa.org/topics/research/multitasking)

---

## 3. Shared Mental Models

**Key finding:** Teams with shared mental models coordinate more effectively, especially when distributed.

Espinosa et al. studied distributed software teams and found that shared mental models — common understanding of the task, the codebase, and who knows what — directly improve coordination and reduce development time. The effect is **stronger for geographically distributed teams** than co-located ones, because distributed teams can't rely on ad-hoc conversations to stay aligned.

**Why this is relevant to bpmux:** The research was conducted on human software teams. Our inference: a shared chat between all participants (humans + AIs) could serve a similar function to the shared mental models Espinosa describes. However, the research does not address whether AIs can participate in or benefit from shared mental models the way humans do. This is an open question, not a validated claim.

**Source:** [Shared Mental Models and Coordination in Large-Scale, Distributed Software Development](https://www.semanticscholar.org/paper/Shared-Mental-Models-and-Coordination-in-Software-Espinosa-Kraut/47fe3d078092e91288a38c2722e6ffdc8755dd1c)

---

## 4. Transactive Memory Systems

**Key finding:** Teams perform better when members know who knows what, and can access each other's expertise.

Transactive memory (Wegner, 1987) is a team's meta-knowledge: not just what the team knows collectively, but who knows which part. High-performing teams have three properties: **specialization** (each member is expert in something different), **credibility** (members trust each other's expertise), and **coordination** (members know who to ask for what). Information technology supports transactive memory by making expertise visible and accessible.

**Why this is relevant to bpmux:** Wegner's TMS framework describes how human teams coordinate through knowledge of "who knows what." bpmux's sender identity and visibility markers are designed with similar intent — making it visible which participant has seen which context. However, TMS research studied human teams, not human-AI teams. Whether sender identity labels and read cursors create the same coordination benefits as genuine transactive memory is our design hypothesis, not a research finding. The analogy is informative for design, but we should not claim TMS research "validates" bpmux's approach.

**Sources:**
- [Transactive Memory Systems — Wegner, 1987](https://en.wikipedia.org/wiki/Transactive_memory)
- [The Impact of IT and TMS on Knowledge Sharing and Team Performance](https://misq.umn.edu/the-impact-of-information-technology-and-transactive-memory-systems-on-knowledge-sharing-application-and-team-performance-a-field-study.html)

---

## 5. Workspace Awareness (CSCW)

**Key finding:** Real-time awareness of collaborators' activities is critical for fluid coordination.

Gutwin and Greenberg's framework for workspace awareness (2002) identifies the elements that collaborators need to know: who is in the workspace, where they are working, what they are doing, what they intend to do next. Shared feedback through a common workspace allows group members to "flexibly and fluidly coordinate their work" without explicit communication.

**Why this is relevant to bpmux:** bpmux's chat is designed as a shared workspace with real-time awareness elements — status bar, visibility markers, inactivity detection. These are inspired by the elements Gutwin and Greenberg identify as enablers of fluid coordination. Whether these elements achieve the same coordination benefits in a human-AI chat as they do in the collaborative editors the research studied is an open question. The research informs our design choices; it doesn't validate them.

**Sources:**
- [A Descriptive Framework of Workspace Awareness for Real-Time Groupware](https://link.springer.com/article/10.1023/A:1021271517844)
- [Awareness and Coordination in Shared Workspaces — Dourish & Bellotti](https://www.semanticscholar.org/paper/Awareness-and-coordination-in-shared-workspaces-Dourish-Bellotti/fc0ccc4e5c0f1f1f1eef1bc9a5a840d1a8e2fd95)

---

## 6. Human-AI Complementarity

**Key finding:** Human-AI teams outperform either alone when roles leverage complementary strengths — but this is rarely achieved in practice.

Research on complementarity in human-AI collaboration (2024-2025) shows that the ideal is "complementary team performance — a level of performance that neither can attain individually." However, this is **rarely observed** in practice, suggesting insufficient understanding of how to design for it. The key: AI automates easy tasks, augments humans on medium tasks, and humans handle hard tasks without AI. Optimized role specification leads to faster decisions, higher quality, and greater satisfaction.

**Why this is relevant to bpmux:** The Ground Control / Major Tom working agreement is a role specification design. The Nature study found positive effects for creation tasks and that "predetermined division of labor" showed promise (only 3 studies explored it). Important caveat: the study's "creation tasks" focused on content generation (text, images). Coding is not explicitly classified in their taxonomy — we infer it fits as a creation task, but the paper does not make this classification. The research suggests predetermined division of labor is a promising direction, not a proven one. Our working agreement is an instance of this understudied approach.

**Sources:**
- [Vaccaro, Almaatouq & Malone (2024) — Nature Human Behaviour](https://www.nature.com/articles/s41562-024-02024-1)
- [MIT Sloan summary](https://mitsloan.mit.edu/press/humans-and-ai-do-they-work-better-together-or-alone)

**Source:** [Complementarity in Human-AI Collaboration: Concept, Sources, and Evidence](https://www.tandfonline.com/doi/full/10.1080/0960085X.2025.2475962)

---

## 7. Verification as the Bottleneck

**Key finding:** In multi-agent AI coding, generation is no longer the bottleneck — verification is.

Addy Osmani's analysis of multi-agent coding patterns (2026) identified that agents produce code faster than it can be verified. The most reliable systems gate transitions (plan → implement → review) rather than just outputs. Quality hooks at each transition prevent low-quality work from compounding.

**Why this is relevant to bpmux:** bpmux's TDD-first approach with human sign-off at every step follows the gating pattern Osmani describes. However, Osmani's observations come from practitioner experience (blog post), not controlled research. The verification-as-bottleneck framing is a widely-shared practitioner insight, not a peer-reviewed finding. It resonates with our design but should be treated as informed opinion.

**Source:** [The Code Agent Orchestra — Addy Osmani](https://addyosmani.com/blog/code-agent-orchestra/) (practitioner blog post, not peer-reviewed research)

---

## Summary: How Research Informs bpmux Design

The research validates that the *problems* bpmux targets are real (context loss, coordination costs, shared understanding). It does not validate that bpmux's *solutions* work — that requires dogfooding. The table below shows how research informed design choices, not how research proves them correct.

| Research Domain | What Research Establishes | bpmux Design Choice It Informed | Evidence Level |
|---|---|---|---|
| Pair programming | Quality improves on complex tasks (meta-analysis) | Target complex engagements | Strong (peer-reviewed) |
| Context switching | 23 min recovery; interrupted tasks take 2x longer (UC Irvine) | Persistent log + read cursor | Strong (peer-reviewed) |
| Shared mental models | Shared understanding improves distributed team coordination (Espinosa) | Shared chat across all participants | Strong (peer-reviewed), but studied human teams, not human-AI |
| Transactive memory | "Who knows what" drives team performance (Wegner, Lewis) | Sender identity, visibility markers | Strong (peer-reviewed), but analogy to AI participants is our inference |
| Workspace awareness | Real-time awareness enables fluid coordination (Gutwin & Greenberg) | Status indicators, inactivity detection | Strong (peer-reviewed), but studied collaborative editors, not AI chat |
| Human-AI complementarity | Predetermined division of labor shows promise; creation tasks positive (Nature 2024) | Ground Control working agreement | Strong (meta-analysis), but only 3 studies on division of labor |
| Verification bottleneck | Generation outpaces review in multi-agent systems (Osmani) | TDD, human sign-off | Practitioner observation (blog post), not peer-reviewed |
