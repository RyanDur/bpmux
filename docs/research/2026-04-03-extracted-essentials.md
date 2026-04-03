# Extracted Essentials: What Survives Into the Morning

*April 3, 2026. Distilled from 9 synthesis, counter-argument, and editor documents across three waves.*

---

## Wave 1: Technology + Architecture

Findings that MUST inform the build:

1. **Do not embed an LLM client inside bpmux.** Four independent analyses (proposal, critique, reconciliation, editor review) all converge. bpmux is the room, not the AI. The external peer model -- a separate bot connecting via WebSocket -- preserves every good idea while keeping bpmux a communication layer. The v1 spec's own principles (general-purpose environment, Bitter Lesson, v1 = communication infrastructure) are unambiguous. This is the most thoroughly validated architectural decision in the entire corpus.

2. **The WebSocket endpoint is the single highest-leverage change.** ~100-150 lines of new Rust on the existing axum server, zero new dependencies. It turns bpmux from something AIs poll into something AIs join. It is the foundation for external bots, the future Web UI, test harnesses, and eventually P2P relay. The broadcast channel must change from `broadcast::channel::<()>` to `broadcast::channel::<Message>` -- a one-line type change that cleans up the internal architecture regardless.

3. **The persistent JSONL log with MCP interface is a genuinely novel primitive.** No existing tool provides a persistent, shared, searchable message log that any AI tool can read from and write to via a standard protocol. Claude Code has CLAUDE.md (write-only, single-user). Cursor has context (per-session, ephemeral). Git has commit messages (lossy summaries after the fact). The log is the product. The bet is on the primitive, not on any single use case.

4. **Solo mode validates independently.** A solo developer whose AI sessions write to a persistent log that future sessions can search has prevented intent debt. No partner needed. No P2P infrastructure needed. This is the irreducible core that stands even if pairing never takes off.

5. **Do not build sophisticated AI behavior yet.** Turn-taking algorithms, behavior trees, motivation scoring, proactivity dials -- all premature. Start with the simplest thing: the bot responds when mentioned by name. The research on AI participation design is excellent and should inform the companion bot later, but it solves a problem that has not been validated.

---

## Wave 2: Psychology + Economics + Behavior

Findings that MUST inform the design:

1. **The active/passive divide is the master variable.** Shared AI reasoning is either helpful or harmful depending entirely on whether the developer engages critically or consumes passively. Seven psychological dimensions converge on this. A developer who passively reads AI reasoning suffers skill atrophy, confidence miscalibration, and destroyed generation effect. The product cannot just make reasoning visible -- it must structurally encourage active engagement or it makes the problem worse.

2. **Information cascades are the highest-severity failure mode.** When the first AI reasoning appears in the shared environment, it frames all subsequent reasoning. Each participant absorbs the first analysis rather than conducting independent analysis. LLMs make this worse due to autoregressive anchoring. The mitigation is specific: require independent reasoning before exposing participants to others' reasoning. bpmux's current ambient-visibility design is the exact opposite of what the research recommends.

3. **Developers cannot accurately perceive their own AI-assisted productivity.** The METR study found developers believed they were 20% faster while actually being 19% slower. The JetBrains telemetry study (800 developers, 2 years) found substantial workflow changes that developers self-report as minimal. This means self-reported satisfaction during dogfooding is unreliable. Measurement must be instrumented, not surveyed.

4. **The review bottleneck is the concrete value proposition.** Faros AI data across 10,000 developers: PR review time increases 91% on high-AI-adoption teams. If bpmux's shared reasoning reduces review time because the reviewer has already seen the reasoning behind the code, that is measurable and high-value. This metric does not depend on Storey's framework or any theory.

5. **Developers share AI conversations deliberately, not casually.** The ICSE 2025 study found developers share to justify, explain, or seek help -- not as ambient context. This challenges the ambient-visibility model. Purposeful sharing (I choose to surface this) may work better than default visibility (everything is visible).

---

## Wave 3: Org Theory + Education + Trust + Philosophy

Findings that MUST inform the vision:

1. **Ambient visibility is the wrong default. Calibrated transparency is the right one.** This is the single most important design finding across all three waves. The transparency paradox (Bernstein): workers less observed are more productive and more innovative. Persistent visibility triggers productivity theater, risk aversion, and legitimized second-guessing. The design needs draft space (private exploration before sharing), layered depth (summaries by default, detail on demand), and temporal boundaries (active session reasoning does not automatically become a permanent record).

2. **The overload ceiling is real and lower than assumed.** Seven organizational frameworks (Conway, Dunbar, Weick, Wenger, Nonaka, Cohen & Levinthal, Argyris) all warn from different directions that more visibility produces diminishing then negative returns. Even active engagement has a throughput limit. Beyond it, developers either disengage (going passive, which is harmful) or burn cognitive budget that should go toward higher-order thinking. Filtering, throttling, and navigability must be designed in from the start, not added later.

3. **The AI can short-circuit human-to-human trust formation.** Trust between humans forms through reciprocal vulnerability. When the AI handles the vulnerable parts of collaboration -- asking "dumb" questions, admitting uncertainty -- it absorbs the vulnerability that should flow between humans. The result: rich human-AI trust, shallow human-to-human trust. bpmux is supposed to strengthen teams, not just human-AI interactions.

4. **Scaffolding must fade.** Every educational framework (Vygotsky, Bruner, Bloom, Kalyuga, CSCL) insists that AI support must decrease as competence increases. Permanent undifferentiated assistance produces dependency. The expertise reversal effect is concrete: the same AI explanation that helps a junior creates measurable drag for a senior (d = -0.428). One level of reasoning visibility cannot serve both.

5. **The tool should be invisible when working, conspicuous only at decision points.** Heidegger's ready-to-hand/present-at-hand distinction is the most actionable design heuristic from the philosophy research. Default to background. Design specific, deliberate breakdowns for conflicts, open decisions, and epistemic warnings. Every interruption must earn its cost.

---

## Cross-Wave Convergence: Highest-Confidence Truths

Three findings that every wave independently confirmed:

1. **The persistent shared log is the core primitive, and it is genuinely novel.** Wave 1 validated this architecturally (no existing tool provides it). Wave 2 validated this economically (review bottleneck gives it a measurable value proposition). Wave 3 validated this theoretically (every organizational and educational framework predicts that externalized shared reasoning changes team dynamics). The log is the product. Everything else is built on top of it.

2. **bpmux must stay a communication layer; the AI must not live inside it.** Wave 1 reached this through architectural analysis and spec alignment. Wave 2 reached this through the observation that the market is solving context loss with smarter solo tools, so bpmux should not compete there. Wave 3 reached this through the Bitter Lesson analysis -- encoding AI behavior into the platform is the domain-specific engineering that general methods outperform. Three independent paths, same conclusion.

3. **Nothing has been validated with real usage, and real usage is the only thing that resolves the competing theoretical predictions.** Wave 1 said: run the Wizard of Oz experiment. Wave 2 said: run the Wizard of Oz experiment, and instrument it because self-reports lie. Wave 3 said: run the Wizard of Oz experiment, and watch specifically for the transparency paradox, information cascades, overload, and vulnerability substitution. Every wave ended at the same place: stop researching, start testing.

---

## Unresolved Dangers: Counter-Arguments That Remain Standing

Three counter-arguments the research did not resolve:

1. **The transparency paradox may be structural, not solvable through calibration.** bpmux's value proposition is visibility. Bernstein's research says visibility degrades performance. Every mitigation (draft space, temporal boundaries, selective sharing) weakens the core value proposition. If reasoning is selectively visible, it is no longer fully shared. If logs are temporary, they do not prevent intent debt. The research treats this as a tension to balance. The counter-argument says it is a contradiction at the heart of the product. The Wizard of Oz experiment must look for productivity theater and performed reasoning as primary signals, not afterthoughts.

2. **Developers may prefer isolation, and absence of supply is not evidence of demand.** Every successful AI tool is single-player. Only 17% of developers say AI improved collaboration. Developers chose remote work, chose async over sync, chose PRs over in-person review. The market is solving context loss by making solo sessions smarter, not by connecting sessions together. The gap argument ("no tool does this, therefore the market wants it") could equally mean the market has spoken: nobody wants it. The research accumulates evidence for the thesis at a ratio of roughly 45:2 compared to evidence against it.

3. **Larger context windows and agentic AI may render bpmux unnecessary.** Claude's context window went from 100K to 1M in a year. The industry is moving toward agents that act autonomously, not toward conversation. bpmux's thesis that developers want to "think with AI" may already be outdated. The Bitter Lesson that bpmux's own research cites says general methods (bigger models, more compute) outperform domain-specific engineering. The counter-argument to the counter-argument -- context windows solve memory but not shared understanding -- is real, but it depends on shared understanding remaining valuable as AI autonomy increases.
