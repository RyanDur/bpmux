# Expertise Externalization: Can Accumulated AI Session Logs Replace Asking the Expert?

**Date**: 2026-04-03
**Hypothesis under investigation**: On a team, as individuals specialize, their AI sessions accumulate domain-specific reasoning. Other team members can query this accumulated knowledge base asynchronously, reducing the need to interrupt the specialist.

---

## 1. Knowledge Externalization in Teams: Does Capturing Expert Reasoning Help Non-Experts?

### The SECI Model (Nonaka & Takeuchi, 1995)

The most influential framework for knowledge externalization is Nonaka and Takeuchi's SECI model (Socialization-Externalization-Combination-Internalization). Externalization -- converting tacit knowledge into explicit forms like documents and concepts -- is considered the most difficult and most important conversion mechanism.

**Key finding**: The empirical basis for the SECI model is contested. Gourlay (2006) and Bratianu (2010) argued that only externalization and internalization are truly knowledge *conversions*; socialization and combination are merely *transfers*. Multiple scholars have questioned the model's generalizability beyond its original Japanese cultural context (Glisby & Holden, 2003). The assumption that tacit knowledge can be straightforwardly converted to explicit knowledge remains debatable.

- Source: https://pmc.ncbi.nlm.nih.gov/articles/PMC6914727/
- Source: https://files01.core.ac.uk/download/pdf/90222.pdf
- Source: https://www.researchgate.net/publication/256657754_A_Critical_Analysis_of_Nonaka's_Model_of_Knowledge_Dynamics

### Expert Knowledge Externalization in Practice

Research on e-learning environments shows that routing learner queries to domain experts can externalize expert tacit knowledge (Ghani, 2009). However, this works through *interactive query-response*, not through browsing pre-existing documentation. The expert must still actively participate in the externalization process.

APQC's 2024 Knowledge Management Priorities report notes that for most organizations, capturing and passing on expertise is a critical priority, especially as experienced workers retire. But the report frames this as an unsolved problem, not a solved one.

- Source: https://www.irrodl.org/index.php/irrodl/article/view/1935
- Source: https://www.apqc.org/blog/2024-knowledge-management-priorities-trends

### The Design Rationale Parallel

Software engineering has a long history of attempting to capture design rationale -- the "why" behind design decisions. Research shows that explicit design rationale is helpful for understanding, improving, and reusing previous designs (Dutoit et al., 2006). The rationale records issues addressed, options considered, and arguments used during design evaluation.

However, design rationale capture has never achieved widespread adoption despite decades of research. The overhead of capture remains a persistent barrier, and retrieval of relevant rationale from large repositories is difficult.

- Source: https://www.sciencedirect.com/science/article/abs/pii/S0164121209001241
- Source: https://link.springer.com/article/10.1007/pl00013715

**Assessment**: The research suggests that captured reasoning *can* help non-experts, but with important caveats: the quality of externalization matters enormously, interactive access outperforms static browsing, and the track record of systems that attempt this at scale is poor.

---

## 2. Organizational Memory and Knowledge Management Systems: The Track Record

### Failure Rates

Knowledge management systems have a well-documented history of failure. While exact percentages vary, the literature consistently describes KM as associated with "countless failures" that have made organizations apprehensive about new implementations.

- Source: https://www.researchgate.net/publication/228585526_Why_Knowledge_Management_Systems_Fail_Enablers_and_Constraints_of_Knowledge_Management_in_Human_Enterprises
- Source: https://link.springer.com/chapter/10.1007/978-3-540-24746-3_30

### Why They Fail: The Major Patterns

**1. Content decay and maintenance burden.** Even strong initial implementations rapidly erode without ongoing maintenance. Budgets get trimmed, champions get reassigned, and the knowledge base fills with stale, duplicate, or irrelevant material. Employees lose trust and stop using the system, creating a death spiral.

**2. Nobody's job.** When "everyone is accountable" for maintaining knowledge, nobody is accountable. Organizations without dedicated KM teams see systems degrade within a few years.

**3. Contribution burden.** Knowledge workers resist systematic attempts to manage and dictate their contributions. The overhead of documenting knowledge on top of doing the actual work is a persistent barrier.

**4. Over-focus on technology.** Many KM failures stem from treating the problem as primarily technological rather than social and cultural. The assumption that deploying a tool will solve knowledge sharing ignores the human dynamics.

**5. Tacit/explicit confusion.** A fundamental misuse of the tacit/explicit knowledge concept -- the assumption that one can easily be converted into the other -- underlies many failed systems.

- Source: https://knowledge-management-tools.net/failure
- Source: https://bloomfire.com/blog/why-knowledge-management-fails/
- Source: https://www.dau.edu/sites/default/files/Migrated/CopDocuments/A_Synthesis_of_Knowledge_Management_Failure_Factors-2014.pdf
- Source: https://ieeexplore.ieee.org/iel5/4085390/4043134/04085406.pdf

### The Wiki vs. Curated Knowledge Base Problem

Unstructured approaches (wikis, chat logs) and curated approaches (knowledge bases) fail in different ways:

- **Wikis** become disorganized and outdated without governance. Anyone can contribute, but nobody curates.
- **Curated knowledge bases** stay accurate but depend on dedicated maintainers and become bottlenecks.
- **Chat and conversation logs** are the worst of both worlds for findability: valuable conversations happen but are siloed in threads that others cannot search or discover after the fact.

Stack Overflow's engineering blog explicitly argued that docs, wikis, and chat clients are *not* knowledge management solutions -- they are information storage, but lack the curation, structure, and maintenance that makes knowledge usable.

- Source: https://stackoverflow.blog/2019/09/30/why-docs-wikis-and-chat-clients-are-not-knowledge-management-solutions/
- Source: https://www.nuclino.com/solutions/wiki-vs-knowledge-base

### Does bpmux's Approach (Conversation Logs) Avoid These Failure Modes?

Conversation logs as a knowledge format have one structural advantage and several structural disadvantages relative to traditional KM:

**Potential advantage**: Zero contribution burden. If reasoning is captured as a byproduct of doing work (AI pair programming sessions), the documentation overhead disappears. This directly addresses failure mode #3 (contribution burden), which is one of the most persistent KM problems.

**Remaining vulnerabilities**:
- Content decay (failure mode #1): Conversation logs become outdated as code evolves. The reasoning in a session from three months ago may reference code that no longer exists.
- Findability: Raw conversation logs are poorly structured for retrieval. A user searching for "why did we choose this database schema" must wade through hours of unrelated dialogue.
- No curation (failure mode #2): Without someone reviewing and validating logs, outdated reasoning persists alongside current reasoning with no way to distinguish them.
- Volume: Conversation logs generate far more text per unit of knowledge than curated documentation, worsening the signal-to-noise ratio.

---

## 3. Transactive Memory Systems (Wegner, 1987)

### The Core Theory

Wegner (1987) proposed that groups develop transactive memory systems (TMS) -- shared knowledge of "who knows what." A TMS consists of individual members' knowledge plus metamemory about others' expertise domains. It enables three key processes:

1. **Directory updating**: Learning what others know
2. **Information allocation**: Routing new information to the right expert
3. **Retrieval coordination**: Knowing who to ask for what

The value of TMS is that it gives individuals access to far more knowledge than they individually possess, improving decision making and information integration.

- Source: https://dtg.sites.fas.harvard.edu/DANWEGNER/pub/Wegner%20Transactive%20Memory.pdf
- Source: https://en.wikipedia.org/wiki/Transactive_memory
- Source: https://journals.aom.org/doi/10.5465/19416520.2011.590300

### Three Dimensions of TMS

Research identifies three critical dimensions (Argote & Ren, 2012):

1. **Specialization**: Differentiated, non-overlapping knowledge domains among members
2. **Credibility**: Trust in the accuracy of others' knowledge
3. **Coordination**: Efficient routing of queries to the right expert

- Source: https://carlsonschool.umn.edu/sites/carlsonschool.umn.edu/files/2018-10/ArgoteRen-JMS-TransactiveMemory-2012.pdf

### Does Making Expertise Queryable Via Logs Change TMS Dynamics?

**Potential positive effects**:
- IT support has a documented positive impact on TMS development in teams (Choi et al., 2010). Technology that makes expertise visible can strengthen directory updating.
- Recent research on human-AI teaming found that accessing information from AI agents is positively linked to generating novel hypotheses and "speaking up" behavior -- but only in higher-performing teams (Bienefeld et al., 2023). The suggestion is that AI agents should be incorporated as knowledge sources within a team's TMS.

**Potential negative effects**:
- Computer-mediated communication studies "usually show a negative influence on the development of a TMS" compared to face-to-face interaction, except in geographically dispersed teams that had no alternative.
- Member turnover disrupts TMS operation. If expertise logs replace the need to interact with the expert directly, the interpersonal credibility dimension of TMS may atrophy.
- A critical risk: if team members believe they can get expertise from logs, they may stop building the "who knows what" directory -- reducing their ability to route truly novel questions to the right person.

- Source: https://pubmed.ncbi.nlm.nih.gov/37599773/
- Source: https://misq.umn.edu/the-impact-of-information-technology-and-transactive-memory-systems-on-knowledge-sharing-application-and-team-performance-a-field-study.html
- Source: https://www.sciencedirect.com/topics/psychology/transactive-memory

### The Credibility Problem

TMS research emphasizes that knowing *who* to trust is as important as knowing *who knows what*. When knowledge is mediated through logs rather than direct interaction, the credibility assessment becomes harder. A team member reading a log cannot easily assess whether the expert was confident, hedging, or exploring a dead end.

- Source: https://mikechristian.web.unc.edu/wp-content/uploads/sites/13307/2016/11/Seigel-Christian-Pearsall-Christian-Ellis-2014.pdf

---

## 4. The Sticky Knowledge Problem (von Hippel, 1994)

### The Core Concept

Von Hippel (1994) defined information stickiness as "the incremental expenditure required to transfer that unit of information to a specified locus in a form usable by a given information seeker." High stickiness means high transfer cost. Tacit knowledge is a subset of sticky knowledge, but not all sticky knowledge is tacit -- some is simply context-dependent or voluminous.

- Source: http://web.mit.edu/people/evhippel/papers/stickyinfo.pdf
- Source: https://evhippel.files.wordpress.com/2013/08/stickyinfo.pdf

### Von Hippel's Five Strategies for Sticky Knowledge

1. Move the knowledge to the problem
2. Move the problem to the knowledge
3. Iterate between knowledge sites
4. Partition the problem so each sub-problem draws on one knowledge locus
5. **Reduce stickiness** through tacit-to-explicit conversion using expert systems and databases

Strategy #5 is what conversation logs attempt. Von Hippel acknowledged this as viable but noted it has limits -- some knowledge resists conversion.

### Szulanski's Empirical Evidence (1996)

Szulanski (1996) studied 271 knowledge transfers across eight companies and found that the major barriers to internal knowledge transfer were:

1. **Recipient's lack of absorptive capacity** (canonical weight: 0.54) -- the recipient lacks the background to understand the transferred knowledge
2. **Causal ambiguity** (0.34) -- it is unclear why something works
3. **Arduous relationship** between source and recipient (0.33)

Critically, motivational factors (unwillingness to share) were *not* the primary barrier. The problem is more about the recipient's ability to absorb than the source's willingness to share.

- Source: https://josephmahoney.web.illinois.edu/BADM%20545_Spring%202008/Paper/Szulanski%20(1996).pdf
- Source: https://sms.onlinelibrary.wiley.com/doi/10.1002/smj.4250171105

### Implications for AI Session Logs

Szulanski's findings cut both ways for the hypothesis:

**Supportive**: If the main barrier is absorptive capacity (not willingness to share), then AI conversation logs could help by providing *explanatory context* alongside decisions. An AI session that reasons through a problem step by step may build absorptive capacity in the reader in ways that terse expert notes do not.

**Challenging**: If causal ambiguity is the second-biggest barrier, conversation logs may not help much. The expert and AI may reach a correct conclusion without fully articulating *why* it works -- the reasoning chain may be present but the causal mechanisms remain ambiguous. The reader gets the "what" but not the deep "why."

---

## 5. AI-Mediated Explanations vs. Human Documentation

### LLM Explanations as Knowledge Artifacts

Research on LLMs for Explainable AI (XAI) shows that LLMs can transform complex outputs into accessible narratives, making technical content more consumable for non-specialists. This is a structural advantage of AI conversation logs over human-written documentation: the AI's explanations tend to be more verbose, more step-by-step, and more explicitly reasoned than what a human expert would write.

- Source: https://arxiv.org/html/2504.00125v1

### Retrieval-Augmented Generation (RAG) Over Conversation History

RAG systems that index conversation history show promising results for knowledge retrieval. Enterprise implementations achieve recall rates above 85% for technical and regulatory documentation. Modern "agentic RAG" systems can use conversation history for context-aware query planning.

However, the enterprise RAG literature is largely in the experimental phase (as of 2024-2025), with 80.5% of implementations relying on standard retrieval frameworks. Effectiveness data for *team conversation log* retrieval specifically is sparse.

- Source: https://www.mdpi.com/2076-3417/16/1/368
- Source: https://pmc.ncbi.nlm.nih.gov/articles/PMC11339511/

### AI Pair Programming Knowledge Transfer

An empirical study compared developer pairs solving programming tasks without AI versus individual developers using GitHub Copilot. The study established that knowledge transfer in pair programming involves closing "knowledge gaps" through exchanging or building new knowledge.

Notably, researchers observed that developers extend AI prompts by asking follow-up questions to deepen understanding -- mimicking how junior developers learn from senior developers. This suggests AI sessions naturally produce explanatory material that could be valuable to later readers.

However, the same research warns that AI-assisted tools could *interfere* with knowledge sharing among teams if they substitute for peer interaction. The recommendation is that AI tools should actively facilitate knowledge sharing and validation with human teammates.

- Source: https://www.se.cs.uni-saarland.de/publications/docs/WSD+.pdf
- Source: https://arxiv.org/pdf/2506.04785

### Searchability Advantage

AI conversation logs have a structural advantage over human documentation for searchability: they contain more natural language, more varied phrasings of concepts, and more explicit reasoning chains. This makes them more amenable to semantic search and RAG retrieval than terse human documentation.

The disadvantage is volume. A 90-minute AI session may contain 20,000+ tokens of dialogue to produce knowledge equivalent to a 500-word design document. The signal-to-noise ratio is unfavorable for browsing.

---

## 6. Counter-Evidence: When Captured Reasoning Misleads

### Stale Information Creates False Confidence

Research consistently shows that outdated information in knowledge bases leads to worse outcomes than having no information at all. One study found 85% of companies blamed stale data for bad decision-making and lost revenue. The mechanism: employees trust documented knowledge, make decisions based on it, and discover too late that it no longer applies.

When employees lose trust in the knowledge base, they stop using it entirely -- making the investment worthless. This is the "death spiral" of knowledge management.

- Source: https://hbr.org/sponsored/2025/04/how-knowledge-mismanagement-is-costing-your-company-millions
- Source: https://knowmax.ai/blog/knowledge-management-challenges/

### Expert Overconfidence in Captured Reasoning

Research on expert judgment shows that experts are more confident than non-experts even when they are wrong, making it difficult to distinguish appropriate confidence from overconfidence in captured reasoning. When experts venture into domains adjacent to but outside their core expertise, they tend to maintain high confidence while making increasingly unreliable claims.

A non-expert reading captured expert reasoning has no way to calibrate which parts reflect genuine expertise and which reflect overconfident extrapolation.

- Source: https://www.nature.com/articles/s41598-022-12847-5
- Source: https://www.psychologytoday.com/us/blog/a-hovercraft-full-of-eels/202404/the-hidden-dangers-of-expert-overconfidence

### The Automation Surprise Parallel

Aviation research on "automation surprise" provides a cautionary parallel. When pilots rely on automated systems, they experience:

- **Out-of-the-loop performance**: Reduced situational awareness from monitoring rather than doing
- **Complacency**: Over-trust in the automated system
- **Skill decay**: Loss of manual control ability from lack of practice

Automation surprises -- where the system behaves differently from expectations -- occur approximately three times per year per pilot. The root cause is not complacency per se, but a lack of deep understanding of the system.

The parallel to querying expertise logs: a team member who relies on captured reasoning rather than building their own understanding may experience "expertise surprise" when the reasoning doesn't apply to their specific situation.

- Source: https://dl.acm.org/doi/10.1145/2702123.2702521
- Source: https://pmc.ncbi.nlm.nih.gov/articles/PMC5633607/
- Source: https://www.mitre.org/sites/default/files/2021-11/pr-16-3426-lessons-lost-nothing-can-go-wrong-automation-induced-complacency.pdf

### Context Dependence of Reasoning

Expert reasoning is deeply contextual. The same expert might reason differently about the same problem given different constraints, deadlines, or team compositions. A captured reasoning chain reflects one specific context. A reader in a different context may apply the reasoning inappropriately.

This connects to Szulanski's "causal ambiguity" -- even when reasoning is captured, the reader may not understand which contextual factors made the reasoning valid and which factors in their own situation differ.

---

## 7. Know-What (Facts) vs. Know-How (Judgment)

### Ryle's Distinction (1949)

Gilbert Ryle's *The Concept of Mind* (1949) established the foundational distinction between "knowing that" (propositional/declarative knowledge) and "knowing how" (procedural knowledge). The majority opinion in academic philosophy is that these are genuinely distinct kinds of knowledge -- knowing how to do something is not reducible to knowing the right facts about how to do it.

A person does not need to verbally articulate their procedural knowledge for it to count as knowledge. Procedural knowledge requires only knowing how to correctly perform an action, not being able to explain it.

- Source: https://plato.stanford.edu/entries/knowledge-how/
- Source: https://en.wikipedia.org/wiki/Procedural_knowledge

### Polanyi's Tacit Dimension (1966)

Michael Polanyi extended this with his observation that "we know more than we can tell." Tacit knowledge is learned through experience and practice rather than formal instruction. Attempts to capture everything in exhaustive documentation fall short because you cannot foresee or codify every relevant detail.

Estimates suggest that only 20-30% of knowledge in a software organization is explicit and documented, while 70-80% is unwritten, experience-based tacit knowledge.

- Source: https://www.billparker.ai/2025/05/polanyis-paradox-why-we-know-more-than.html
- Source: https://art-of-information.com/2024/05/08/tacit-knowledge/

### Can AI Conversation Logs Capture Judgment?

AI conversation logs capture a *trace* of judgment -- the sequence of decisions, the options considered, the reasoning articulated. This is more than traditional documentation captures. But it is still not judgment itself.

**What logs capture well (know-what)**:
- What decision was made
- What alternatives were considered
- What trade-offs were articulated
- What information was consulted

**What logs capture poorly (know-how)**:
- *Why* the expert's intuition flagged a particular approach as risky
- The pattern recognition that led the expert to consider certain options and ignore others
- The calibration of confidence -- how much the expert trusted their own reasoning
- The contextual factors the expert unconsciously weighted

An analogy: a chess game transcript captures every move but not the grandmaster's intuition about board position. A reader can study the moves but cannot acquire the judgment that produced them without extensive practice of their own.

### The Absorptive Capacity Connection

This connects back to Szulanski's finding that absorptive capacity is the primary barrier to knowledge transfer. A conversation log may provide the reasoning chain, but the reader needs sufficient domain background to *make sense of* that reasoning. Without it, the log is noise -- or worse, a source of false confidence.

---

## 8. The Interruption Cost: The Problem Being Solved

### The Scale of Developer Interruptions

Research by Gloria Mark (UC Irvine) shows that knowledge workers are interrupted every 6-12 minutes on average, and it takes an average of 23 minutes and 15 seconds to fully regain focus after an interruption. A single unplanned context switch can consume up to 20% of a developer's cognitive capacity.

Research also shows that interrupted tasks take twice as long and contain twice as many errors as uninterrupted tasks. Developers facing frequent interruptions report higher stress, mental fatigue, and time pressure.

- Source: https://super-productivity.com/blog/context-switching-costs-for-developers/
- Source: https://jellyfish.co/library/developer-productivity/context-switching/

### Knowledge Silos and Bus Factor

Knowledge silos -- where critical information stays trapped with one person -- create organizational fragility measured by the "bus factor." The cost is real and quantifiable: estimates suggest organizations lose $50K+ per developer annually to context-switching costs alone.

- Source: https://plantheflow.com/blog/knowledge-silos-cost/
- Source: https://dev.to/teamcamp/the-hidden-cost-of-developer-context-switching-why-it-leaders-are-losing-50k-per-developer-1p2j

### Relevance to the Hypothesis

The interruption cost literature validates the *problem* the hypothesis addresses. Interrupting a specialist to ask a question is genuinely expensive for both parties. Any mechanism that reduces interruptions while preserving knowledge transfer has real value -- *if* the knowledge transfer is effective.

---

## Summary: What the Evidence Says

### Supporting the hypothesis:
1. **Zero contribution burden** is a genuine structural advantage. The #1 reason KM systems fail is the cost of contributing. If knowledge capture is a byproduct of work, this barrier disappears.
2. **AI explanations are more verbose and step-by-step** than human documentation, potentially building absorptive capacity in readers.
3. **The interruption cost is real and large.** Any reduction in expert interruptions has measurable value.
4. **IT-supported expertise directories improve TMS** in teams, and AI session logs could function as a richer form of expertise directory.
5. **Design rationale capture research** shows that "why" documentation is genuinely valuable for understanding and reusing previous designs.

### Challenging the hypothesis:
1. **Most knowledge management systems fail**, and conversation logs share several failure modes: content decay, no curation, poor signal-to-noise ratio.
2. **Tacit knowledge and judgment resist externalization.** Logs capture reasoning traces but not the intuition, pattern recognition, and confidence calibration that constitute expert judgment. The 70-80% of knowledge that is tacit remains tacit.
3. **Absorptive capacity is the primary barrier** to knowledge transfer (Szulanski, 1996). If the reader lacks domain background, captured reasoning won't help -- and may create false confidence.
4. **Stale information is worse than no information.** Conversation logs from months ago may reference code and decisions that no longer apply. Without curation, outdated reasoning persists alongside current reasoning.
5. **TMS credibility may erode.** If team members substitute log-querying for expert interaction, they lose the ability to assess credibility and route truly novel questions.
6. **The automation surprise parallel** suggests that relying on externalized expertise without building one's own understanding leads to failures when the externalized reasoning doesn't apply.
7. **Volume problem.** Conversation logs have terrible signal-to-noise ratios compared to curated documentation. The knowledge exists in the logs but finding it is a needle-in-a-haystack problem.

### The central tension:
The hypothesis conflates two different things: reducing interruptions (which the evidence supports as valuable) and effective knowledge transfer through logs (which the evidence suggests is much harder than it appears). The research consistently shows that **interactive** knowledge exchange outperforms **static** knowledge retrieval. An AI conversation log is static -- the original expert's judgment is frozen in time and context. The reader gets the expert's past reasoning but not the expert's ability to adapt that reasoning to the reader's current situation.

The most honest reading of the evidence: conversation logs from AI sessions may be useful as a **starting point** that reduces the number and duration of interruptions, but they are unlikely to **replace** the need for expert interaction. They may shift interruptions from "explain everything from scratch" to "I read your session log and have specific questions" -- which is a genuine improvement, but a more modest one than the hypothesis suggests.
