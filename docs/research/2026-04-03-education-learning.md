# Educational Theory and Shared AI Reasoning

How established learning theories predict what happens when AI reasoning is shared across a team rather than consumed privately. This document covers Vygotsky, Bruner, Lave & Wenger, Schon, Bloom, Kalyuga, and CSCL research -- all applied to the specific question bpmux raises: does making AI reasoning visible and persistent change learning outcomes for developers?

Note: The generation effect and cognitive load theory (Sweller) are covered in `developer-psychology.md`. This document builds on adjacent but distinct educational frameworks.

---

## 1. Vygotsky's Zone of Proximal Development

### The Theory

Lev Vygotsky (1978) defined the Zone of Proximal Development (ZPD) as the distance between what a learner can do independently and what they can do with guidance from a "more knowledgeable other" (MKO). Learning happens most effectively inside this zone -- tasks too easy produce no growth, tasks too hard produce frustration, but tasks just beyond current ability with appropriate support produce development.

The MKO does not need to be a teacher. It can be a peer, a tool, or -- as recent research argues -- an AI system.

### AI as the More Knowledgeable Other

A 2025 systematic literature review of 158 empirical studies (2021-2024) examined how AI tools assist students in identifying and operating within their ZPD. The review found that AI tools help learners personalize self-assessment, improve motivation and engagement, and lead to better academic performance. Generative AI can fulfill the criteria of the MKO in a social constructivist framework by scaffolding learning and providing an augmented ZPD.

But there is a critical caveat. A 2025 paper introduced the concept of the "Zone of No Development" (ZND) -- a state in which continuous AI assistance replaces cognitive struggle and impedes intellectual autonomy. When the MKO never withdraws, the learner never internalizes the skill. The ZPD collapses into dependency.

**Sources:**
- [ERIC: Exploring the Impact of Integrating AI Tools in Higher Education Using the ZPD](https://eric.ed.gov/?q=from+AND+web&ff1=subTechnology+Uses+in+Education&id=EJ1467947) -- peer-reviewed, 158 studies
- [The Unspoken Crisis of Learning: The Surging Zone of No Development](https://arxiv.org/html/2511.12822v1) -- arXiv 2025
- [Generative AI: the 'More Knowledgeable Other' in Medical Education](https://pmc.ncbi.nlm.nih.gov/articles/PMC12254308/) -- PMC, peer-reviewed

### What Changes When AI Reasoning Is Shared

Private AI assistance (Claude Code, Copilot) places the AI as each developer's personal MKO. The developer operates within their individual ZPD with invisible support. This is effective for the individual but has two problems:

1. **Nobody else knows the ZPD shifted.** A junior developer who solves a complex problem with heavy AI scaffolding appears to have the same competence as one who solved it independently. The team cannot calibrate who actually understands what.

2. **The ZND risk is invisible.** If a developer is in the Zone of No Development -- accepting AI output without cognitive engagement -- nobody can tell from the outside. The code works. The understanding is absent.

Shared AI reasoning (bpmux) changes both dynamics:

- **The scaffolding is visible.** When the team can see that the AI walked a developer through a solution step by step, they know the developer is operating in their ZPD, not beyond it. This is useful information for code review, task assignment, and mentoring.

- **The ZND becomes detectable.** If the chat log shows a developer accepting AI suggestions without questions, without pushback, without articulating their own understanding, that pattern is visible to the team. Private AI use hides passivity. Shared AI use exposes it.

- **The ZPD of the team shifts.** When two developers and an AI reason together, the collective ZPD is larger than any individual's. Problems that neither developer could solve alone, and that neither could solve with private AI assistance alone, may become solvable when reasoning is pooled in a shared environment.

### The Hard Question

Does shared AI reasoning expand ZPDs or create dependency? The answer almost certainly depends on the interaction pattern. If shared reasoning encourages developers to articulate their thinking before receiving AI input (preserving the productive struggle), it expands the ZPD. If it provides a stream of ready-made reasoning to passively absorb, it creates a collective ZND.

---

## 2. Scaffolding Theory (Bruner)

### The Theory

Wood, Bruner, and Ross (1976) formalized scaffolding as temporary support that enables a learner to accomplish a task they could not complete independently. The critical feature: scaffolding is designed to be removed. The scaffold exists to make the learner capable of performing without it. If the scaffold becomes permanent, it is not scaffolding -- it is a crutch.

Bruner identified key scaffolding behaviors: recruiting interest in the task, reducing degrees of freedom (simplifying the problem), maintaining direction, marking critical features, controlling frustration, and demonstrating solutions.

### The Fading Problem

The most important -- and most difficult -- aspect of scaffolding is fading: the gradual removal of support as the learner gains competence. A 2025 study on collaborative programming education (Zhang, 2025) tested a three-stage fade-out scaffolding model and found that effective fading improved both programming achievement and self-efficacy. But the study also found that poorly calibrated fading (removing support too early or too late) produced worse outcomes than no scaffolding at all.

Research on AI-driven scaffolding identifies a persistent challenge: static scaffolding and ineffective calibrated fading. AI systems tend to provide the same level of support regardless of whether the learner has progressed. A 2025 study on metacognition in programming education found that easy access to AI-generated solutions reduces engagement in metacognitive practices -- a phenomenon the authors call "metacognitive laziness."

**Sources:**
- [Zhang (2025): Three-Stage Fade-Out Scaffolding for Collaborative Programming](https://onlinelibrary.wiley.com/doi/abs/10.1111/jcal.70012) -- peer-reviewed
- [Scaffolding Metacognition in Programming Education](https://arxiv.org/html/2511.04144v1) -- arXiv 2025
- [AI-Driven Visual Scaffolding in Education: A Comprehensive Literature Review](https://rsisinternational.org/journals/ijrsi/articles/ai-driven-visual-scaffolding-in-education-a-comprehensive-literature-review/) -- 2024

### Is Shared AI Reasoning a Scaffold?

Current AI coding tools function as scaffolding that never fades. Copilot suggests code on every keystroke. Claude Code answers every question. There is no mechanism for the tool to say "you should be able to do this yourself now" and step back. The support is permanent and undifferentiated.

bpmux's shared reasoning introduces a natural fading mechanism that private AI tools lack: social awareness of competence growth.

**How shared reasoning enables fading:**

- **Visible progression.** Over time, a team's chat logs show a developer asking fewer basic questions and engaging with more complex problems. This trajectory is visible to the developer themselves and to their team, creating natural pressure to reduce reliance.

- **Peer calibration.** In a shared environment, a senior developer can observe that a junior colleague is ready for less scaffolding and adjust their interaction accordingly. "Try working through this one before asking the AI" is natural in a group conversation but impossible in a private AI session.

- **Self-awareness.** When developers see their own history of AI interactions alongside their teammates', they gain perspective on their reliance patterns. Private AI use creates no mirror. Shared AI use does.

**The risk:** Shared reasoning could also prevent fading. If the team norm becomes "always share AI reasoning," developers may continue consuming scaffolding they no longer need because it is always present in the channel. The scaffolding becomes environmental rather than intentional.

### Design Implication

bpmux should support intentional fading. This could mean: modes where AI reasoning is less verbose for experienced team members, the ability for a developer to say "I want to try this without AI help" and have that be visible and respected, or team norms that explicitly discuss when to lean on shared AI reasoning and when to work independently.

---

## 3. Situated Learning (Lave & Wenger)

### The Theory

Jean Lave and Etienne Wenger (1991) proposed that learning is not an isolated cognitive process but is fundamentally situated in social participation. Their model of "legitimate peripheral participation" (LPP) describes how newcomers join a community of practice by starting at the periphery -- observing, doing simple tasks, absorbing norms -- and gradually moving toward full participation.

The key insight: newcomers learn not through formal instruction but through increasingly meaningful participation in authentic practice. A junior developer does not learn software engineering from textbooks. They learn by reading real code, participating in real code reviews, hearing real design discussions, and gradually taking on more responsibility.

### LPP in Software Development

Research on open source communities has directly applied Lave and Wenger's framework. Studies on newcomer onboarding in OSS projects found that initial conditions (skills, experience) did not effectively predict long-term participation. What mattered was situated learning and identity construction -- the degree to which newcomers could meaningfully observe and participate in the community's actual practice.

The barriers to LPP in software teams are well-documented: newcomers cannot observe expert reasoning because it happens in private conversations, private AI sessions, and individual mental models. They see the outputs (code, pull requests, design documents) but not the process (the reasoning, the tradeoffs considered, the alternatives rejected).

**Sources:**
- [Lave & Wenger (1991): Situated Learning: Legitimate Peripheral Participation](https://www.cambridge.org/highereducation/books/situated-learning/6915ABD21C8E4619F750A4D4ACA616CD) -- Cambridge University Press
- [Understanding Sustained Participation in Open Source Software Projects](https://dl.acm.org/doi/10.2753/MIS0742-1222250401) -- peer-reviewed
- [Attracting, Onboarding, and Retaining Newcomer Developers in OSS Projects](https://www.researchgate.net/publication/260677091_Attracting_Onboarding_and_Retaining_Newcomer_Developers_in_Open_Source_Software_Projects) -- peer-reviewed

### Can Juniors Learn from bpmux Logs?

This is one of the strongest theoretical arguments for shared AI reasoning. Today, a junior developer joining a team sees:

- The codebase (the artifact, not the reasoning)
- Pull request descriptions (summaries of decisions, not the deliberation)
- Commit messages (what changed, not why the alternatives were rejected)

They cannot see: the design discussion, the tradeoffs weighed, the AI's suggestions that were rejected and why, the moment of uncertainty resolved through reasoning. All of this happens in private sessions and ephemeral conversations.

bpmux logs would contain exactly the kind of situated practice that LPP theory says newcomers need. A junior reading a log of a senior developer and AI reasoning through an architecture decision would witness:

- How experts frame problems (what they notice, what they ignore)
- How experts evaluate AI suggestions (what they accept, what they push back on, what questions they ask)
- How experts make tradeoffs (what they prioritize, what they sacrifice)
- The vocabulary and reasoning patterns of expert practice

This is not formal instruction. It is exactly what Lave and Wenger describe: observing authentic practice from the periphery and gradually building the schemas to participate in it.

### The Apprenticeship Analogy

Traditional software apprenticeship (pair programming, code review, mentoring) works because the junior observes the senior's reasoning in real time. But this is expensive -- it requires synchronous time from the senior developer. bpmux logs create an asynchronous approximation: the senior's reasoning is captured in the chat log and available to the junior at any time.

This is not a perfect substitute. Real-time mentoring involves back-and-forth, questions, and calibration to the junior's level. A log is a one-directional record. But it is far richer than the current alternative, which is no access to expert reasoning at all.

### The Limitation

LPP theory emphasizes that learners need to move from peripheral to full participation. Observing logs is peripheral participation. The question is whether bpmux creates pathways from observation to active contribution. Can a junior who has been reading logs begin participating in the shared chat? Does the environment support a gradual increase in participation, or is it binary (observer vs. full participant)?

---

## 4. Reflective Practice (Schon)

### The Theory

Donald Schon (1983) distinguished two forms of professional reflection:

- **Reflection-in-action:** Thinking while doing. The professional encounters a surprise, reframes the situation on the fly, and adjusts their approach. This is the expert programmer who notices something "feels wrong" about a data flow and restructures their approach mid-implementation.

- **Reflection-on-action:** Thinking after doing. The professional looks back at what happened, analyzes what worked and what did not, and builds new understanding. This is the retrospective, the post-incident review, the "what did we learn" conversation.

Schon argued that expert practitioners develop a "repertoire of exemplars" -- a library of past situations and responses that they draw on when facing new problems. This repertoire is built through repeated cycles of action and reflection.

### The Problem with AI-Assisted Development

Current AI tooling disrupts both forms of reflection:

**Reflection-in-action is short-circuited.** When a developer encounters a surprise -- unexpected behavior, an unfamiliar API, a design that does not fit -- the natural response is to pause, think, and reframe. AI coding assistants offer an alternative: ask the AI. The pause-think-reframe cycle is replaced with prompt-receive-apply. The developer gets an answer faster but builds no repertoire.

**Reflection-on-action has no material.** After a coding session with private AI assistance, what is there to reflect on? The code exists. The AI conversation is ephemeral or buried in session history. The decisions, the alternatives considered, the reasoning paths explored and abandoned -- all of this is lost. There is nothing substantive to reflect on except the final artifact.

### How Persistent Shared Reasoning Enables Reflection

bpmux's persistent chat log is a reflection artifact. It captures exactly the material Schon says professionals need for reflective practice:

**For reflection-on-action:**

- The full reasoning chain is preserved. Not just what was decided, but how, why, and what alternatives were considered.
- The team can review a session's log together, seeing not just the outcome but the process. This is directly analogous to Schon's case study method, where professionals learn from analyzing detailed records of practice.
- Patterns become visible over time. A developer who reviews their own logs may notice recurring blind spots, habitual approaches that limit their thinking, or moments where they consistently deferred to AI without engaging critically.

**For reflection-in-action:**

- The shared chat creates natural pauses. Instead of immediately asking the AI for a solution, the developer articulates their thinking in the group conversation. This articulation is itself a reflective act -- putting confused thoughts into words forces them into structure.
- Other participants' questions serve as reflection prompts. "Why are you approaching it that way?" from a human partner is a real-time invitation to reflect-in-action.

A 2025 study on AI agents and reflective practices found that AI tools can transform traditional methods of reflection in learning, but only when the tools are designed to prompt reflection rather than replace it. The study found that socially shared regulation of learning improved when AI agents facilitated rather than directed the reflective process.

**Source:**
- [How AI Agents Transform Reflective Practices: A Three-Semester Comparative Study](https://www.sciencedirect.com/science/article/abs/pii/S0920548925001230) -- ScienceDirect, peer-reviewed
- [Protecting Human Cognition in the Age of AI](https://arxiv.org/html/2502.12447v3) -- arXiv 2025

### The Repertoire Problem

Schon's repertoire of exemplars is built through experience and reflection. AI threatens this in two ways:

1. **Reduced variety of experience.** If AI consistently provides the same patterns and approaches (as it tends to), the developer's repertoire narrows to "what the AI usually suggests" rather than the diverse set of approaches built through personal experimentation.

2. **False repertoire.** A developer who has seen the AI solve a problem may believe they have added it to their repertoire, but recognizing a solution is not the same as being able to generate it. The next time they face a similar problem without AI, the repertoire fails.

Shared reasoning partially mitigates the first problem: when multiple developers contribute to the conversation, the diversity of approaches increases. The AI is not the only voice. But the second problem remains: even shared reasoning does not guarantee deep encoding into the individual's repertoire.

---

## 5. Bloom's Taxonomy Applied to AI Assistance

### The Framework

Bloom's taxonomy (1956, revised by Anderson & Krathwohl, 2001) arranges cognitive processes in ascending order of complexity:

1. **Remember** -- recall facts, terms, basic concepts
2. **Understand** -- explain ideas, summarize, interpret
3. **Apply** -- use knowledge in new situations
4. **Analyze** -- break information into parts, find patterns, identify relationships
5. **Evaluate** -- justify decisions, make judgments, critique
6. **Create** -- produce new work, design, construct

### Where AI Excels and Where It Fails

A 2025 framework (BloomAPR) applied Bloom's taxonomy to evaluate LLM capabilities in automated program repair and found a clear pattern: AI performs well at lower cognitive levels and degrades at higher ones. Research on AI-generated educational content found the same: AI aligns well with lower-order cognitive tasks but has limitations addressing evaluation and creation.

Applied to AI coding assistance:

| Bloom's Level | AI Capability | Developer Risk |
|---|---|---|
| Remember | Excellent. AI recalls syntax, APIs, patterns instantly. | Developer stops memorizing anything, loses basic fluency. |
| Understand | Good. AI can explain code, summarize behavior. | Developer stops building their own mental models. |
| Apply | Good. AI can use patterns in new contexts. | Developer becomes dependent on AI for implementation. |
| Analyze | Mixed. AI can identify some patterns but misses systemic issues. | Developer's analytical muscles atrophy from disuse. |
| Evaluate | Weak. AI cannot reliably judge tradeoffs in context. | This is where developer judgment remains critical. |
| Create | Weak for novel design. AI recombines known patterns. | True architectural creativity still requires human cognition. |

The concern is not that AI is bad at higher levels -- it is that AI's strength at lower levels reduces the practice that develops higher-level skills. A developer who never has to remember syntax, explain behavior, or apply patterns in new situations has fewer opportunities to build the foundation for analysis, evaluation, and creation.

**Sources:**
- [BloomAPR: A Bloom's Taxonomy-based Framework for LLM-Powered APR](https://arxiv.org/html/2509.25465v1) -- arXiv 2025
- [Cultivating Independent Thinkers: AI, Bloom's Taxonomy and Critical Thinking](https://link.springer.com/article/10.1007/s10639-025-13476-x) -- Springer, peer-reviewed
- [Using Bloom's Taxonomy to Understand AI Adoption in Higher Education](https://onlinelearningconsortium.org/olc-insights/2025/10/blooms-for-ai-adoption/) -- OLC 2025

### Does Shared Reasoning Push Developers Up or Down?

**The downward pressure:** If shared AI reasoning provides ready-made analysis, evaluation, and design rationale, developers consume higher-order thinking rather than producing it. The AI analyzes the tradeoffs, evaluates the options, and proposes the design. Developers at all levels of the taxonomy receive answers instead of developing the skills to generate them.

**The upward pressure:** Shared reasoning can push developers up the taxonomy if the interaction structure demands it. Consider:

- The AI provides information at the Remember/Understand levels. ("This API supports these three authentication methods. Here is how each works.")
- The shared conversation forces the team to operate at the Analyze/Evaluate levels. ("Which method fits our security requirements and performance constraints?")
- The persistent log captures the Create level output. ("Here is the architecture we designed through this deliberation, and why.")

In this model, AI handles the lower levels of the taxonomy so that humans can focus on the higher levels. This is the optimistic case. But it only works if the interaction pattern demands human engagement at higher levels rather than allowing AI to operate across all levels.

### The Taxonomy Inversion Hypothesis

There is an argument that shared AI reasoning inverts the traditional skill development path. Traditionally, developers build up through the taxonomy: memorize syntax, understand patterns, apply them, analyze systems, evaluate tradeoffs, create designs. AI allows developers to start at the top: create and evaluate with AI handling the lower levels.

This inversion could be beneficial (focusing human cognition where it is most needed) or harmful (building a house without a foundation). The research does not yet resolve this question. The honest answer is that it depends on whether the developer still builds lower-level understanding through engagement with the AI's output, or whether they skip it entirely.

---

## 6. The Expertise Reversal Effect

### The Theory

The expertise reversal effect (Kalyuga et al., 2003; Sweller, 2010) is the finding that instructional techniques effective for novices can become ineffective or harmful for experts. The mechanism: information that helps a novice build a schema is redundant for an expert who already has that schema. Processing redundant information consumes working memory without producing learning, creating extraneous cognitive load.

A 2024-2025 meta-analysis (60 studies, 5,924 participants) confirmed the effect and quantified it:

- Low prior knowledge learners benefit from high-assistance instruction (effect size d = 0.505).
- High prior knowledge learners benefit from low-assistance instruction (effect size d = -0.428).
- The effect is asymmetric: providing novices with assistance has a stronger positive effect than the negative effect of providing assistance to experts.

**Source:**
- [A Cornerstone of Adaptivity: A Meta-Analysis of the Expertise Reversal Effect](https://www.sciencedirect.com/science/article/pii/S0959475225000660) -- ScienceDirect, peer-reviewed, 2025

### Applied to Shared AI Reasoning

If bpmux shows the same AI reasoning to all team members regardless of expertise level, it runs directly into the expertise reversal effect:

**For junior developers:** Visible AI reasoning that explains its approach, surfaces relevant concepts, and walks through the logic step by step is exactly the scaffolding novices need. It reduces intrinsic load, provides schema-building material, and operates within the ZPD. The research predicts significant benefit.

**For senior developers:** The same reasoning is redundant. A senior developer who already has the schema for, say, concurrent data access patterns does not need the AI to explain the tradeoffs between mutexes and channels. Processing that explanation consumes working memory without producing learning. The research predicts this creates drag -- not catastrophic, but a persistent, low-grade extraneous load that accumulates over a workday.

**The asymmetry matters.** The meta-analysis found that the benefit to novices from assistance is larger than the harm to experts. This suggests that if bpmux must choose a default level of AI verbosity, erring toward more explanation (novice-friendly) produces better net outcomes than erring toward less (expert-friendly). But this is a blunt instrument.

### What This Means for Mixed-Experience Teams

Real teams have mixed experience levels. The expertise reversal effect means a single level of AI reasoning visibility will be simultaneously too much for some and too little for others. This is not a solvable problem at the content level -- you cannot write one explanation that is both scaffolding for a novice and non-redundant for an expert.

**Possible mitigations:**

- **Layered reasoning.** AI provides a brief summary by default (sufficient for experts) with expandable detail (available for those who need scaffolding). The shared chat contains the summary; the detail is available on demand.

- **Adaptive verbosity.** The AI adjusts its explanation depth based on who it is addressing or based on explicit team configuration. This is technically feasible but socially complicated -- it creates visible evidence of who the system considers "junior" and "senior."

- **Cultural norms over technical solutions.** Teams develop norms about when to ask for explanation ("Can you walk me through that?") rather than having explanation imposed by default. This preserves senior developers' efficiency while keeping scaffolding available for those who need it.

### The Expert Benefit That the Theory Misses

The expertise reversal effect assumes experts gain nothing from the information. But in a shared AI reasoning context, experts gain something the theory does not account for: awareness of what the AI told their junior colleague. Even if the expert does not need the explanation for their own understanding, knowing what the junior was told helps the expert calibrate expectations, identify potential misunderstandings, and provide more targeted mentoring.

This is not a learning benefit in the traditional sense. It is a coordination benefit that operates on a different axis than the expertise reversal effect measures.

---

## 7. Collaborative Learning with AI (CSCL Perspective)

### The Field

Computer Supported Collaborative Learning (CSCL) is a 30-year research tradition studying how technology mediates collaborative learning. The fundamental CSCL question is not "does technology help individuals learn?" but "does technology help groups learn together?" This is precisely bpmux's question.

### What CSCL Research Says About AI in Groups

A 2023 paper in the International Journal of Computer-Supported Collaborative Learning examined knowledge co-construction with generative AI from a CSCL perspective. The core finding: argumentative dialogs among humans and AI tools can achieve joint knowledge construction with appropriate prompts, where emergent processes take place. But the paper also warned that AI participation can disrupt the social dynamics that make collaborative learning effective if not carefully designed.

A 2025 study on AI-Agent-supported collaborative learning in university programming courses proposed an "AI-CL" approach and found that AI agents showed potential in enhancing collaborative learning outcomes. Traditional CSCL methods had limitations that AI agents could address -- particularly in providing consistent, patient support for group work that human facilitators could not scale.

**Sources:**
- [Co-constructing Knowledge with Generative AI: Reflections from a CSCL Perspective](https://link.springer.com/article/10.1007/s11412-023-09409-w) -- ijCSCL, peer-reviewed
- [Impact of AI-Agent-Supported Collaborative Learning on Programming Learning Outcomes](https://link.springer.com/article/10.1007/s10639-025-13487-8) -- Springer, peer-reviewed 2025

### Is Two People + AI Better Than One Person + AI?

The research points toward yes, but with important conditions.

**Arguments from CSCL research that favor group + AI:**

1. **Explanatory knowledge building.** CSCL research consistently finds that the act of explaining one's thinking to a collaborator deepens understanding (the "self-explanation effect"). AI alone does not trigger this -- you do not need to explain yourself to Claude Code. But a shared environment with human collaborators does.

2. **Productive disagreement.** When two developers and an AI reason together, disagreements are more likely to surface. CSCL research finds that cognitive conflict (encountering a perspective that contradicts your own) is one of the strongest drivers of learning -- provided it occurs in a psychologically safe environment. A single developer working with AI is unlikely to experience productive disagreement because the AI tends to accommodate rather than challenge.

3. **Role differentiation.** CSCL studies on role assignment find that designated roles (summarizer, questioner, devil's advocate) improve collaborative learning outcomes. In a two-person + AI setting, natural roles emerge: one person drives, one navigates, the AI provides information and perspective. This role structure is absent in one-person + AI interaction.

4. **Social accountability.** Research on the "audience effect" in CSCL shows that learners produce better explanations and engage more deeply when they know others will see their work. A developer who must articulate their reasoning in a shared chat puts more cognitive effort into that articulation than one typing a prompt into a private AI session.

**Arguments that temper the optimism:**

1. **Coordination overhead.** CSCL research acknowledges that collaborative learning incurs coordination costs. Managing turn-taking, resolving disagreements, and maintaining shared understanding all consume cognitive resources. Adding AI to the group increases coordination complexity.

2. **Social loafing.** In groups, individuals sometimes contribute less because they assume others (including the AI) will carry the load. The "free rider" effect is well-documented in CSCL research. AI may exacerbate this: "The AI will catch any issues, so I don't need to review as carefully."

3. **Dominance dynamics.** CSCL research finds that one participant often dominates group interaction. In a developer + developer + AI setting, there is a risk that the AI becomes the dominant voice, and both humans defer to it rather than engaging with each other's reasoning.

### The CSCL Design Principles That Apply

Thirty years of CSCL research have produced design principles for effective technology-mediated collaborative learning. The principles most relevant to bpmux:

- **Structure the collaboration.** Unstructured group work produces worse outcomes than structured collaboration with defined roles, turn-taking protocols, and shared goals. bpmux should provide lightweight structure (explicit turns, agreement mechanisms) rather than leaving interaction entirely freeform.

- **Make thinking visible.** CSCL tools that externalize reasoning (concept maps, shared whiteboards, annotated diagrams) produce better learning outcomes than those that only share final products. bpmux's chat log is inherently a thinking-visible medium, which aligns with this principle.

- **Support knowledge integration, not just knowledge sharing.** Sharing information is necessary but not sufficient. Effective collaborative learning requires participants to integrate different perspectives into a shared understanding. The AI's role should include synthesis ("It sounds like Developer A is concerned about performance while Developer B is focused on maintainability -- here is how both concerns might be addressed") rather than just providing information.

---

## Synthesis: What Educational Theory Predicts for bpmux

| Theory | Prediction for Shared AI Reasoning | Design Implication |
|---|---|---|
| Vygotsky (ZPD) | Expands the collective zone but risks creating a Zone of No Development if scaffolding is permanent | Build in mechanisms for fading -- the AI should do less as the team demonstrates competence |
| Bruner (Scaffolding) | Shared reasoning is scaffolding, but scaffolding without intentional fading becomes dependency | Support explicit "I want to try this without AI" modes; make reliance patterns visible |
| Lave & Wenger (Situated Learning) | Chat logs are legitimate peripheral participation artifacts; juniors can learn from senior+AI sessions | Prioritize log readability and searchability as a learning resource, not just a communication record |
| Schon (Reflective Practice) | Persistent logs enable reflection-on-action that ephemeral AI sessions cannot | Design for retrospective review, not just real-time conversation |
| Bloom (Taxonomy) | AI handles lower cognitive levels; shared reasoning can push humans to higher levels if interaction demands it | Structure interactions so humans operate at Analyze/Evaluate/Create while AI handles Remember/Understand/Apply |
| Kalyuga (Expertise Reversal) | Same AI reasoning helps juniors but creates extraneous load for seniors | Layer reasoning depth; brief summaries by default, expandable detail on demand |
| CSCL Research | Two people + AI likely produces better learning than one person + AI, but only with structured interaction | Provide lightweight collaboration structure; support roles, turn-taking, and explicit agreement |

### The Overarching Pattern

Every theory reviewed here converges on the same structural insight: **the value of shared AI reasoning depends entirely on whether it promotes active cognitive engagement or enables passive consumption.**

- Vygotsky: Active engagement operates in the ZPD; passivity creates the ZND.
- Bruner: Active engagement uses scaffolding as intended; passivity prevents fading.
- Lave & Wenger: Active peripheral participation builds practice; passive observation builds nothing.
- Schon: Active reflection builds repertoire; passive receipt builds false confidence.
- Bloom: Active engagement pushes up the taxonomy; passivity collapses it to the lowest levels.
- Kalyuga: Active experts filter redundant information; passive exposure creates load.
- CSCL: Active collaboration produces knowledge; passive group membership produces social loafing.

bpmux's design challenge is not "make AI reasoning visible." It is "make AI reasoning visible in a way that structurally demands active engagement from every participant." The educational theory is remarkably consistent on this point: visibility without engagement is not just useless -- it is harmful. It creates the illusion of learning while preventing actual learning from occurring.
