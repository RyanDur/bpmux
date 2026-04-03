# Wave 2 Synthesis: What Changed

*April 2, 2026. Covers only new findings from the five wave-2 research documents. Does not repeat wave-1 conclusions.*

---

## 1. What Wave 2 Found That Changes Our Understanding

### The active/passive divide is the master variable

Wave 1 established that shared AI reasoning is valuable. Wave 2 discovered that this is wrong as stated. The developer psychology research found that **the same feature -- visible AI reasoning -- is either harmful or helpful depending entirely on whether the developer's relationship to it is active or passive**. Seven independent psychological dimensions (flow state, psychological safety, Dunning-Kruger, decision fatigue, paradox of choice, cognitive load, generation effect) all converge on this single finding.

This is not a nuance. It is a fundamental constraint on the product. A developer who reads AI reasoning critically and generates their own understanding before absorbing the AI's benefits from bpmux. A developer who passively consumes AI reasoning is actively harmed by it -- skill atrophy accelerates, confidence miscalibrates upward, and the generation effect (remembering things better when you produce them yourself) is destroyed.

Wave 1 said: make reasoning visible. Wave 2 says: make reasoning visible *and structurally encourage active engagement with it*, or you are making the problem worse.

### The empirical data is more sobering than the theory

Wave 1 built its case on frameworks (Storey, Hutchins, stigmergy) and analogies (Bletchley Park, mycelium). Wave 2 brought empirical data, and the data complicates the narrative:

- **Developers do not perceive their own behavior changes.** The JetBrains telemetry study (800 developers, 2 years) found substantial workflow changes that developers self-report as minimal. The METR study found developers believed they were 20% faster while actually being 19% slower. Self-reports and telemetry are not interchangeable. This means we cannot trust developer enthusiasm for bpmux as evidence that it works. We need instrumented measurement.

- **Only 17% of developers say AI improved team collaboration.** The Stack Overflow 2025 survey (65,000+ respondents) found this is the lowest-rated AI impact by a wide margin. Developers are not asking for collaboration tools. They are asking for better individual tools. The collaboration problem nobody is solving might also be the collaboration problem nobody wants solved.

- **Sharing AI conversations is deliberate and purposeful, not casual.** The ICSE 2025 study of ChatGPT conversations in pull requests found developers share to justify, explain, or seek help -- not as ambient context. This challenges bpmux's ambient visibility model. Developers may want purposeful sharing (I choose to show you this reasoning) rather than default visibility (everything I do is visible).

- **The review bottleneck is the real crisis.** Faros AI data across 10,000 developers: PR review time increases 91% on high-AI-adoption teams. Developers complete 21% more tasks but the review burden absorbs the gains. The actual bottleneck in AI-assisted teams is not code generation or even context loss -- it is that nobody can review fast enough. This reframes bpmux's value proposition: if shared reasoning reduces the review burden by frontloading context, that is a measurable, high-value outcome.

### Information cascades are the highest-severity failure mode

Wave 1 identified risks (automation bias, overreliance, log growth). Wave 2 found a failure mode that is structurally more dangerous: **information cascades**. When the first AI reasoning appears in the shared environment, it frames all subsequent reasoning. Each participant reads the first analysis, finds it plausible, and builds on it rather than conducting independent analysis. The apparent consensus of three is really the judgment of one.

This is worse than anchoring bias because cascades are self-reinforcing and self-concealing. Participants do not know they are in a cascade. They believe they independently confirmed the first analysis when they merely absorbed it. LLMs make this worse -- their autoregressive architecture means they structurally anchor on prior reasoning, not just psychologically.

The mitigation is clear and consistent across the failure modes research: **require independent reasoning before exposing participants to others' reasoning**. This is the single highest-leverage design decision, and bpmux's current design (ambient visibility of all reasoning) is the exact opposite of what the research recommends.

---

## 2. New Risks Surfaced

### Risks wave 1 did not identify

**AI brain fry.** A March 2026 BCG study found that 14% of workers experience mental fatigue from excessive AI oversight. Productivity peaks at 1-3 AI tools and declines at 4+. bpmux adds another information stream to evaluate. Every message is a micro-decision: read or skip, act on or ignore, agree or challenge. This is the oversight burden by another name.

**The consensus trap.** Shared visible reasoning accelerates convergence. Teams reach agreement faster -- but premature consensus kills innovation by closing the solution space before alternatives are explored. Research found that dialectical inquiry and devil's advocacy produce higher quality decisions than consensus. Assigned devil's advocates are ineffective (people discount them); only authentic dissent improves decisions. AI participants will join emerging consensus rather than challenge it unless specifically instructed otherwise.

**Performed reasoning vs. actual reasoning.** When people know their reasoning is observed, they change how they reason. The Hawthorne effect applied to thinking. A developer working alone might think "I have no idea what this code does." The same developer in a shared environment writes "Analyzing the interaction between the authentication module and the session handler." The first is honest and productive. The second is polished and useless. The AI makes this asymmetry worse: its reasoning always looks clean and confident, making human reasoning look messy by comparison.

**Attribution erosion over time.** The most original thinkers are systematically under-credited when spillovers are high (others building on your reasoning). Incremental refiners get disproportionate visibility. This does not matter in week one. It matters in month six, when the person who consistently frames problems correctly stops bothering because nobody notices.

### Risks wave 1 identified that wave 2 made worse

**Groupthink.** Wave 1 noted the risk. Wave 2 found that LLMs are trained on text reflecting deference and agreeableness, making them echo chamber amplifiers in shared environments. The AI dimension makes groupthink structurally harder to prevent.

**Free riding.** Wave 1 noted it abstractly. Wave 2 found the paradox: visibility discourages overt free riding but encourages performative contribution -- reasoning that looks like work but adds no independent signal. AI participants exhibit the functional equivalent: producing derivative reasoning that paraphrases others rather than conducting independent analysis.

---

## 3. New Opportunities Surfaced

### The scrutiny gap

The most directly relevant empirical finding for bpmux: developers accept AI suggestions with less scrutiny when alone but engage critically when a human partner is present. In sessions with both a human and AI available, developers prioritize human interaction over AI interaction. A shared AI session could restore the scrutiny that solo AI use erodes. This is not theoretical. It is observed behavior from a controlled study comparing human pairs with AI copilots.

### The review bottleneck as a value metric

If bpmux's shared reasoning reduces PR review time -- because the reviewer has already seen the reasoning behind the code -- this is a measurable, high-value outcome that maps directly to the Faros AI finding (91% longer reviews on high-AI-adoption teams). This gives bpmux a concrete, quantifiable value proposition that does not depend on Storey's framework or any theory at all: "teams using bpmux review PRs X% faster because the reasoning is already visible."

### The market gap nobody is requesting

Stack Overflow data contains a telling absence: only 17% say AI improved collaboration, but this is not a top complaint. Developers ask for better individual tools. The actual bottleneck is collaborative (review overload). The problem developers experience is collaborative, but the solution they request is individual. This is a genuine market gap -- but it means bpmux cannot rely on pull demand. It must create demand by demonstrating value that developers did not know they needed.

### The consultancy wedge is stronger than expected

The economics research found that consultancies have specific dynamics that make bpmux unusually valuable: time-bounded engagements with rotating developers, client confidentiality requirements that favor P2P architecture, knowledge transfer as a persistent pain point, and the persistent log as a deliverable ("here is the reasoning log from our engagement"). The buying process is decentralized -- tech leads decide, not procurement. The P2P architecture with local-first data makes security review easy. This is a real go-to-market path, not a theoretical one.

---

## 4. Does Wave 2 Strengthen or Weaken the Case?

**Both, in specific ways.**

### Strengthened

- The isolation problem is confirmed by empirical data (Faros AI, METR, JetBrains telemetry, Stack Overflow), not just theory. The evidence base is now multi-source and convergent.
- The scrutiny gap finding provides direct empirical support for bpmux's core mechanism: making AI sessions observable restores critical engagement.
- The review bottleneck gives bpmux a concrete, measurable value proposition independent of any framework.
- The consultancy use case has clearer economic logic than the general developer market.
- The 12-18 month market window assessment is grounded in competitive analysis, not speculation.

### Weakened

- The failure modes are more severe than wave 1 acknowledged. Information cascades and premature consensus are high-severity, high-likelihood risks that bpmux's current design (ambient visibility) actively enables.
- The active/passive divide means bpmux needs to structurally encourage active engagement -- it cannot just make reasoning visible and hope for the best.
- The empirical data shows developers do not want collaboration tools (17% say AI improved collaboration). bpmux is solving a problem the market is not asking to have solved.
- The perception-reality gap (developers cannot accurately assess their own AI-assisted productivity) means self-reported bpmux satisfaction during dogfooding is unreliable evidence.

### Net assessment

The case for the problem is stronger. The case for bpmux as the solution is more conditional than wave 1 suggested. The product is not automatically valuable because it makes reasoning visible. It is valuable only if the design actively prevents the failure modes that visibility creates.

---

## 5. The Counter-Arguments You Need to Hear

The wave-2 counter-arguments document is the most important document in the entire wave-2 corpus. It challenges claims that the rest of the research accepted without pushback. The strongest challenges:

**The null hypothesis was never stated.** No document seriously considers the possibility that bpmux provides no value. Every document starts from the premise that shared AI reasoning is valuable. What if AI reasoning is like compiler output -- useful for the person running it, meaningless to everyone else?

**The cost of the intervention is never quantified.** Every collaboration tool imposes costs: setup, maintenance, context switching, social pressure to participate, cognitive load from monitoring. If bpmux adds 30 minutes of overhead per day and saves 23 minutes, it is a net loss. The research catalogs benefits exhaustively but never quantifies costs.

**Developers may prefer isolation.** The research treats isolation as unambiguous negative. But developers chose remote work, chose async over sync, chose PRs over in-person review. AI tools accelerate something developers already wanted: fewer interruptions and fewer forced conversations.

**The Storey framework is one month old.** The entire value proposition rests on a March 2026 preprint that has not been replicated or independently validated. This is a real risk to the narrative, not a minor caveat.

**The stigmergy analogy breaks at the joints that matter.** Pheromones evaporate (JSONL logs do not). Ants are interchangeable (developers are not). Ants do not evaluate trace quality (developers must). Calling a chat log "stigmergic" does not add explanatory power beyond "people leave messages and others read them."

**Absence of supply is not evidence of demand.** The gap argument ("no tool does this, therefore the market wants it") is circular. The market may have spoken: developers want better solo tools, not collaboration tools.

**The competition is not standing still.** By the time bpmux ships a usable v1, Cursor may have shared AI chat, Claude Code may have extended Agent Teams, and the gap may not exist.

**The autonomy trend may make bpmux obsolete.** The market is moving toward more AI autonomy (Devin, agentic Claude Code, SWE-bench agents), not toward more conversation. bpmux's thesis that developers want to "think with AI" may be a 2024 insight already outdated by 2026.

These are not reasons to stop. They are reasons to test hypotheses before committing to infrastructure. The counter-arguments document's closing observation is the sharpest: the research has spent its energy finding reasons bpmux should exist at a ratio of roughly 45:2 compared to finding reasons it might fail. That ratio should concern you.

---

## 6. Updated Recommendation

Wave 1 said: build the WebSocket endpoint, run the Wizard of Oz experiment. That recommendation still holds. Wave 2 adds constraints and sharpens priorities.

### Design constraints from wave 2

1. **Independence before convergence.** The most important design principle from wave 2. Do not show all reasoning to all participants by default. Support a pattern where participants form independent assessments before seeing others' reasoning. This directly mitigates information cascades, groupthink, and premature consensus.

2. **Pull over push.** Reasoning visibility should be available on demand, not forced on attention. The flow state research and decision fatigue research both point to the same conclusion: ambient information that demands no attention is fine; ambient information that creates micro-decisions is harmful.

3. **Active engagement mechanisms.** The active/passive divide means the product needs structural nudges toward engagement: conversational turn-taking where the developer states intent before the AI responds, explicit agreement mechanisms, prompts for the developer to articulate their reasoning before seeing the AI's.

4. **Measure with instrumentation, not self-reports.** The perception-reality gap is too large to rely on developer satisfaction surveys. Instrument the dogfooding: token costs, time-in-log, decision alignment with prior context, review time with and without bpmux context.

### What to test first

The Wizard of Oz experiment remains the right first step, but wave 2 adds a specific thing to watch for: **does the interaction produce independent reasoning or cascade-following?** If the human reads the AI's reasoning and simply agrees, that is a cascade. If the human reads it and pushes back, elaborates, or generates an alternative, that is active engagement. The ratio between these two modes is the single most important signal from the experiment.

### The one new thing to add to the plan

**Test purposeful sharing vs. ambient visibility.** The ICSE 2025 finding that developers share AI conversations deliberately (to justify, explain, seek help) challenges the ambient visibility model. Consider a mode where reasoning is captured but not displayed by default -- participants choose what to surface. This adds friction (bad for ambient awareness) but reduces noise (good for preventing cascades and decision fatigue). The right answer is probably not all-or-nothing, but the experiment should explore where on that spectrum the value lies.

### What has not changed

- Build the WebSocket endpoint. The foundation for everything.
- Do not embed an LLM client inside bpmux. Still correct.
- Cheap experiments before production architecture. Even more correct now that wave 2 revealed failure modes the design must account for.
- The persistent log with MCP interface is the core primitive. Wave 2 did not challenge this. It challenged how the log is surfaced, not whether it should exist.

### The honest bottom line

Wave 2 moved the assessment from "bpmux solves a real problem" to "bpmux solves a real problem, but the obvious implementation (make everything visible to everyone) creates new problems that are as severe as the ones it solves." The path forward requires more design sophistication than wave 1 implied. Ambient visibility is not enough. The product must be opinionated about *how* reasoning is shared, not just *that* it is shared.

The counter-arguments are strong. The failure modes are real. The market signal is ambiguous (developers want better solo tools, not collaboration tools). The theoretical framework is young. None of this is fatal. All of it demands that the first experiment be designed not just to validate the thesis but to genuinely look for evidence against it. If the Wizard of Oz session produces nothing but cascade-following and performed reasoning, the honest response is to redesign, not to explain away the result.

The problem is real. The solution is conditional. Test the conditions.
