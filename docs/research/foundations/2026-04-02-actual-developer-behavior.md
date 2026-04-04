# What Developers Actually Do with AI: Empirical Evidence

Research date: 2026-04-02

Prior research in this directory draws heavily on theory, analogies, and studies of other collaborative systems. This document focuses exclusively on empirical data about what developers actually do when using AI tools in collaborative settings — observational studies, telemetry, interaction logs, and behavioral data.

## 1. How Developers Actually Use AI in Teams (Observational Data)

### The METR Study: Screen Recordings of Real Work

The most rigorous observational study to date comes from METR (2025). They recruited 16 experienced open-source developers working on repositories averaging 22k+ stars and 1M+ lines of code. Developers completed real issues (246 total) while recording their screens, producing 143 hours of manually labeled screen recordings.

**The headline finding: developers using AI took 19% longer to complete tasks.**

But the perception data is what matters for bpmux: developers predicted AI would speed them up by 24%, and even after experiencing the slowdown, they still believed AI had sped them up by 20%. The gap between what developers think they are doing and what they are actually doing is enormous.

A follow-up experiment had significant dropout — developers refused to participate in conditions where they could not use AI — suggesting that AI use has become identity-bound, not just practical.

Sources: [METR Blog](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/), [arXiv 2507.09089](https://arxiv.org/abs/2507.09089)

### "They Control, Not Vibe": Field Observations of Professional Developers

A UC San Diego / Cornell study (2025) conducted field observations (N=13) and qualitative surveys (N=99) watching experienced developers use AI coding agents. The observational sessions ran August-October 2025, with developers working on self-chosen tasks while researchers observed and asked them to think aloud.

**What professionals actually do:**
- Plan the work themselves, then give agents very specific instructions, context, and limits
- Make agents work in small chunks, then verify by running tests, running the app, and reviewing changes
- Mention software quality attributes (correctness, readability, maintainability) nearly twice as often as productivity gains
- Want speed AND quality, not speed OR quality — this dual requirement drives controlling behavior

**What agents are actually used for:** starter code, tests, documentation, simple refactors, small bug fixes. They are not used for complex logic, deep business rules, fitting into old codebases, security-sensitive work, or design decisions.

This matters for bpmux: professionals are already treating AI as a constrained collaborator that needs oversight. The question is whether that oversight can be shared.

Source: [arXiv 2512.14012](https://arxiv.org/abs/2512.14012)

### JetBrains Telemetry: Two Years of IDE Behavior (800 Developers)

The "Evolving with AI" study (Sergeyuk et al., ICSE 2026) analyzed two years of fine-grained telemetry from 800 developers (October 2022 to October 2024), combined with a survey of 62 professionals.

**What telemetry reveals vs. what developers report:**
- AI users produce substantially more code but also delete significantly more
- Survey respondents report productivity gains and perceive minimal changes in other dimensions
- But telemetry shows more substantive behavioral changes than developers self-report
- The authors conclude that behavioral telemetry and survey responses are "not interchangeable measures of the same construct, but complementary windows into developer experience"

The pattern of increased output coupled with increased deletion suggests that AI shifts the developer role toward curation — write more, delete more, edit more. This is a workflow change developers do not perceive themselves as having made.

Source: [arXiv 2601.10258](https://arxiv.org/abs/2601.10258)

## 2. What Happens When Two Developers Share an AI Session

### Human Pairs vs. AI Copilots: Knowledge Transfer Compared

The most directly relevant study is "From Developer Pairs to AI Copilots" (2025), which compared developer pairs solving a programming task without AI against individual developers using GitHub Copilot.

**Key findings:**
- Similar frequency of knowledge transfer episodes across both settings
- Developers accept Copilot suggestions with less scrutiny than human partner suggestions
- Human-human pair programming enables spontaneous interactions but increases distraction risk
- Knowledge transfer with Copilot is less likely to be aborted, but suggestions are accepted with less scrutiny
- In pair programming sessions with both a human partner and AI available, developers prioritize human interaction over AI interaction

**The scrutiny gap is critical for bpmux:** when a human partner suggests something, developers engage critically. When AI suggests the same thing, they accept more passively. A shared AI session could potentially restore the scrutiny that solo AI use erodes.

Source: [arXiv 2506.04785](https://arxiv.org/abs/2506.04785)

### AI in Pair Programming: Classroom-Scale Data

A quasi-experimental study (2023-2024) with 234 undergraduate students compared AI-assisted pair programming (using GPT-3.5 Turbo and Claude 3 Opus) with human-human pair programming and individual programming.

AI-assisted pair programming significantly increased intrinsic motivation and reduced programming anxiety, with outcomes comparable to human-human pairing. AI-assisted groups outperformed both individual and human-human groups in programming tasks. However, this was a classroom setting, not professional practice.

Source: [STEM Education Journal](https://stemeducationjournal.springeropen.com/articles/10.1186/s40594-025-00537-3)

## 3. What Developers Share About AI Interactions

### The ICSE 2025 Study: ChatGPT Conversations in Pull Requests

This is the closest empirical data to the bpmux use case. Hao & Hasan (ICSE 2025) analyzed 210 and 370 developers' shared ChatGPT conversations in GitHub pull requests and issues, respectively.

**What developers actually share:**
- Conversations span 16 types of software engineering inquiries
- Most frequently shared: code generation, conceptual questions, how-to guides, issue resolution, code review
- 33.2% of conversations shared in PRs and 36.9% in issues are multi-turn (not just a single prompt)
- Developers use shared conversations to facilitate role-specific contributions — PR authors, code reviewers, and issue collaborators share for different reasons

**Why they share:**
- To justify design decisions
- To provide context for code changes
- To ask for help debugging an AI interaction that went wrong
- To demonstrate approach before implementing

**Sharing is deliberate and purposeful, not casual.** Developers do not share AI conversations the way they share links. They share them as evidence — to justify, explain, or seek help. This suggests bpmux needs to support purposeful sharing, not ambient visibility.

Source: [arXiv 2403.10468](https://arxiv.org/abs/2403.10468), [Springer](https://link.springer.com/article/10.1007/s10664-024-10540-x)

## 4. Vicarious AI Learning: The "Over the Shoulder" Effect

### No Direct Studies Found

There are no published empirical studies specifically on vicarious learning from watching another developer use AI. This is a genuine research gap. The closest analogues:

- The "From Developer Pairs to AI Copilots" study shows that developers prioritize human interaction when a human is present, suggesting that observing someone else's AI use might be more instructive than using AI oneself
- The Anthropic skill formation study (below) found that developers who used AI for conceptual inquiry rather than delegation scored dramatically higher, suggesting that how you engage with AI matters more than whether you use it
- Think-aloud protocol studies exist for AI creativity tasks but not specifically for developers watching other developers use AI coding tools

**This is an opportunity for bpmux.** If no one has studied what happens when developers watch each other use AI, and the evidence suggests that scrutiny and engagement style dramatically affect outcomes, then a tool that makes AI sessions observable could provide research data as well as practical value.

## 5. GitHub Copilot Telemetry: What the Data Shows

### Scale and Adoption

- 15 million users by early 2025 (4x increase in one year)
- 90% of Fortune 100 companies adopted Copilot
- Jellyfish analysis of 2M+ PRs: AI-assisted PR percentage grew steadily throughout 2025
- Code assistant adoption increased from 49.2% in January to 69% in October 2025 (Jellyfish)
- 89% tool retention rate at 20 weeks for Copilot/Cursor users (Jellyfish)

### What the Telemetry Reveals About Collaboration

- Copilot code review has been used on 60 million code reviews
- In 71% of reviews, Copilot surfaces actionable feedback; in 29%, it says nothing
- PR review time increases 91% on teams with high AI adoption (Faros AI, 10,000 developers across 1,255 teams) — AI-generated code creates a review bottleneck
- Developers on high-AI-adoption teams complete 21% more tasks and merge 98% more PRs, but the review burden absorbs the gains

**The collaboration bottleneck:** AI accelerates code production but not code review. Individual output increases but organizational throughput does not. This is the central paradox that Faros AI documented across 10,000 developers: developers say they are faster, but delivery velocity does not improve.

Sources: [Faros AI Report](https://www.faros.ai/blog/ai-software-engineering), [Jellyfish 2025 Metrics](https://jellyfish.co/blog/2025-ai-metrics-in-review/), [GitHub Blog](https://github.blog/ai-and-ml/github-copilot/60-million-copilot-code-reviews-and-counting/)

### Only 17% Say AI Improved Team Collaboration

The Stack Overflow 2025 Developer Survey (65,000+ respondents) asked about AI agent impact. Approximately 70% agree agents reduced time on specific tasks and 69% agree agents increased productivity. But only 17% agree that agents improved collaboration within their team — the lowest-rated impact by a wide margin.

Source: [Stack Overflow 2025 Survey](https://survey.stackoverflow.co/2025/ai)

## 6. The Gap Between Tool Adoption and Workflow Change

### The Perception-Reality Divide

Multiple independent datasets converge on the same finding: developers believe AI makes them faster, but measured data shows otherwise or shows much smaller gains than perceived.

| Study | Perceived Gain | Measured Reality |
|-------|---------------|-----------------|
| METR (2025) | +24% expected, +20% believed after | -19% actual (slower) |
| Faros AI (2025) | Developers report working faster | No measurable improvement in delivery velocity |
| JetBrains telemetry (2024) | Minimal perceived workflow change | Substantial behavioral changes in telemetry |
| Stack Overflow (2025) | 69% say increased productivity | Trust in AI accuracy fell from 40% to 29% |

### The Curation Tax

The JetBrains telemetry data reveals the mechanism: developers write more and delete more. The Faros AI data adds context: 66% of developers say their biggest frustration is AI solutions that are "almost right, but not quite," and 45% say debugging AI-generated code is more time-consuming than writing it themselves.

The editing and validation burden intensifies rather than disappearing. Developers must allocate attention to specification, verification, and integration tasks. They have adopted the tool but not changed their workflow to account for its actual costs.

### AI Hasn't Changed How Teams Work

A two-wave longitudinal interview study (N=15) at a project-based software development organization (2023-2025) found:

- In 2023, developers envisioned AI as an intelligent coordinator that could align projects, track progress, and ease interpersonal frictions
- By 2025, AI was used mainly to accelerate individual tasks (coding, writing, documentation)
- Core collaboration problems — performance accountability, fragile communication — remained unresolved
- AI reshaped culture: efficiency became a norm, transparency about AI use became a marker of professionalism
- But the structural problems of distributed, ad hoc software teams persisted entirely

Source: [arXiv 2509.10956](https://arxiv.org/abs/2509.10956)

## 7. What Developers Ask For vs. What They Need

### The Skill Formation Trap

Anthropic (2026) studied 52 junior software engineers learning a new Python library. Participants using AI scored 17% lower on comprehension quizzes than those who coded by hand. But the critical finding was about usage patterns:

- Developers who used AI for conceptual inquiry scored 65%+
- Developers who delegated code generation to AI scored below 40%
- Cognitive effort — even getting painfully stuck — is important for fostering mastery

**Developers ask for:** faster code generation, less boilerplate, autocomplete everything.
**What they actually need:** engagement that preserves understanding.

Source: [Anthropic Research](https://www.anthropic.com/research/AI-assistance-coding-skills), [arXiv 2601.20245](https://arxiv.org/abs/2601.20245)

### The Trust Collapse

Stack Overflow 2025 data shows a trust crisis:
- 84% of developers use or plan to use AI tools
- But only 29% trust AI answers (down from 40%)
- 75.3% say they don't trust AI answers
- 61.7% have ethical or security concerns about generated code
- 61.3% say they want to fully understand their code

**Developers ask for:** more powerful AI features, better models, faster responses.
**What they actually need:** reasons to trust the output, shared verification, peer validation.

Source: [Stack Overflow 2025 Survey](https://survey.stackoverflow.co/2025/)

### The Collaboration Request Nobody Makes

The Stack Overflow data contains a telling absence: only 17% say AI improved team collaboration, yet this is not a top complaint. Developers are not asking for better AI collaboration tools. They are asking for better individual AI tools.

But the Faros AI data shows the actual bottleneck is collaborative — PR review time increases 91% on high-AI-adoption teams. The problem developers are experiencing is collaborative (review overload), but the solution they are requesting is individual (better code generation).

## Implications for bpmux

### What the empirical data supports

1. **Shared scrutiny matters.** Developers accept AI suggestions with less scrutiny when alone but engage critically when a human partner is present. Making AI sessions observable could restore the scrutiny that solo use erodes.

2. **Sharing is purposeful, not ambient.** Developers share AI conversations deliberately — to justify, explain, or seek help. bpmux should support purposeful sharing, not just passive visibility.

3. **The real bottleneck is review, not generation.** AI accelerates code production but review absorbs all the gains. A tool that makes AI reasoning transparent could reduce the review burden by frontloading context.

4. **Developers do not perceive their own behavior changes.** Telemetry shows substantial workflow changes that developers self-report as minimal. Externally visible AI sessions could make invisible behavior changes visible.

5. **Engagement style determines learning outcomes.** Delegation to AI impairs learning; conceptual engagement preserves it. A shared environment could nudge toward engagement over delegation through social accountability.

6. **The collaboration problem nobody is solving.** Developers ask for better individual tools. The actual bottleneck is collaborative. This is a genuine market gap — not because competitors are failing, but because developers are not asking for the right thing.

### What remains unknown

- No studies on vicarious AI learning (watching someone else use AI)
- No studies on shared AI sessions between two professional developers
- No studies on how AI transparency affects code review burden
- No studies on whether social presence changes AI delegation patterns in professional settings
- No longitudinal data on teams that adopted shared AI practices vs. teams that did not
