# Full Corpus Distillation v2

*April 3, 2026. Synthesized from 60+ research documents across three waves, multiple editorial reviews, counter-arguments, a self-critique, and a corpus-level critique. This version strips shipping pressure, corrects confidence levels, and removes decorative framing.*

---

## 1. The Problem

AI coding tools optimize individual throughput while eroding team coherence. The evidence for this converges from independent sources:

**Empirical findings (directly measured):**
- GitClear: code cloning up 4x, refactoring down from 25% to under 10% (2024-2025 data)
- Faros AI (10,000 developers): teams with high AI adoption merge 98% more PRs, but PR review time increases 91%
- METR study: experienced open-source developers were 19% slower with AI while believing they were 20% faster
- LeadDev survey (2025): 54% of engineering leaders plan to hire fewer juniors
- JetBrains telemetry (800 developers, 2 years): substantial workflow changes that developers self-report as minimal

**Conceptual frameworks (named but not yet validated empirically):**
- Storey et al. (arXiv:2603.22106, March 2026) define cognitive debt (erosion of shared understanding) and intent debt (absence of externalized rationale). This is a one-month-old preprint. The concepts name real intuitions but have not been empirically tested or replicated. bpmux's thesis depends on these concepts being real.

**The mechanism:** When a developer has a conversation with an AI about an architectural decision, the reasoning evaporates when the session ends. The code lands. The rationale is gone. This happens for every developer, every day. No tool currently captures this reasoning in a way that is persistent, shared, and searchable.

**What is NOT evidence for the problem:** Civilization research (cuneiform, printing press), nature research (ants, mycelium), crisis coordination (NASA, ICS), and military doctrine (mission command, OODA loops). These are analogies for solution patterns. They do not establish that AI coding tools erode team coherence. Developer behavior data does.

**Counter-evidence the corpus underweighted:**
- Nature Human Behaviour meta-analysis (Vaccaro et al., 2024): human-AI combinations perform *worse* than the best of either alone on decision-making tasks. The corpus cited the positive result for "creation tasks" in supporting documents and the negative result only in risk documents. A fair reading: the evidence that adding AI to a shared environment improves outcomes is mixed.
- Only 17% of developers in the Stack Overflow 2025 survey said AI improved team collaboration. The corpus does not investigate what the 83% tried or why it failed.

---

## 2. What bpmux Is

**bpmux is a persistent, shared message log with an MCP interface that any AI tool can read from and write to.**

That is what exists. That is the product primitive.

What exists today: a solo TUI chat with JSONL persistence and an MCP server (send_message, read_messages) over HTTP. One working feature.

**What makes this potentially novel:** No existing tool provides this combination. Claude Code has CLAUDE.md (write-only, single-user). Cursor has context (per-session, ephemeral). Slack has threads (not designed for AI consumption). Git has commit messages (lossy summaries after the fact). The JSONL log with MCP tools is a new primitive — if it turns out to be useful. That "if" has not been tested.

**What bpmux is not (high confidence — multiple independent analyses):**
- Not an orchestrator or AI framework
- Not an embedded LLM client (bpmux is the room, not the AI)
- Not a surveillance tool
- Not a tool execution environment
- Not a replacement for any existing AI coding tool

**Organizational frameworks cited in the corpus (Conway, Nonaka, Weick, Argyris, Cohen-Levinthal, Wenger, Dunbar):** These are lenses, not validations. Any shared communication tool satisfies them. Slack satisfies them. A Google Doc satisfies them. They do not distinguish bpmux from existing tools. Included here for completeness, not as evidence.

---

## 3. Design Hypotheses

These are working principles, not proven facts. Each needs to be tested through usage. Calling them "proven" (as the prior distillation did) creates the illusion that testing has already happened.

### 3.1 Persist reasoning, not just outcomes

The code tells you what. The log tells you why. Without the why, every future change is made in the dark.

**Evidence level:** Strong theoretical motivation (Storey's Triple Debt Model, ADR literature, developer experience anecdotes). Zero empirical evidence that persisting AI reasoning in a shared log reduces cognitive or intent debt. This is the core hypothesis of the entire project.

### 3.2 The AI must not live inside the tool

bpmux is the room, not the AI. The external peer model (WebSocket endpoint, separate bot process) keeps bpmux a communication layer.

**Evidence level:** Sound architectural judgment consistent with the spec's original intent. The Bitter Lesson supports general-purpose infrastructure over encoded behavior. The conclusion was also reached because MCP's technical limitations forced it, and because it requires far less code than the alternative. Convenience does not make it wrong, but the speed of conviction was influenced by the effort differential. Four documents "independently" reaching this conclusion were written in the same session — they are one analysis, not four.

### 3.3 Solo mode validates independently

A solo developer whose AI sessions write to a persistent, searchable log has prevented intent debt. No partner needed.

**Evidence level:** Reasonable product hypothesis. Untested. Whether a solo developer actually uses a persistent log to review AI reasoning from prior sessions is an empirical question.

### 3.4 Token awareness and bounded context are architectural

Models degrade long before hitting context limits (Databricks, Chroma, NVIDIA RULER studies).

**Evidence level:** This is the one principle with genuine empirical backing from independent third-party research. The connection to "bound the read window" is a reasonable engineering decision grounded in measured behavior.

### 3.5 Transparency requires a product decision

The user's original design: "Everything is visible to everyone. There are no private channels or hidden messages."

The research recommends: calibrated transparency, draft space, selective sharing, temporal boundaries.

These directly contradict each other. The research has taken a side — against the user's original design — without asking. This is not a calibration challenge to be solved with a four-tier rendering model. It is a product decision that has not been made.

The transparency paradox (Bernstein 2012): workers who are less observed are more productive and more innovative. If this applies to bpmux, the core value proposition (visibility) actively harms the outcome it aims to improve. The research treats this as manageable through design. The counter-argument says it may be structural — inherent to any visibility tool.

---

## 4. Unresolved Contradictions

These are genuine tensions that cannot be resolved by more research. Each represents a fork in the road where different evidence points different directions.

### 4.1 Transparency vs. Performance

bpmux's value is visibility. Bernstein's research says visibility degrades performance. Every mitigation (draft space, temporal boundaries, selective sharing) weakens the core value proposition. If reasoning is selectively visible, it is no longer fully shared. If logs are temporary, they do not prevent intent debt. This may be a contradiction at the heart of the product, not a tension to balance.

### 4.2 Active Engagement vs. Passive Consumption

Visible AI reasoning is helpful when engagement is active and harmful when it is passive (skill atrophy, confidence miscalibration, destroyed generation effect). Seven psychological dimensions converge on this. bpmux provides visibility. It cannot provide engagement. This is a risk the product carries.

### 4.3 Independence vs. Convergence (Information Cascades)

When the first AI reasoning appears in a shared environment, it frames all subsequent reasoning. Each participant absorbs the first analysis rather than conducting independent analysis. LLMs worsen this through autoregressive anchoring. The mitigation (independent reasoning before convergence) contradicts the transparency principle. This is the highest-severity failure mode identified in the research.

### 4.4 Shared Understanding vs. Overload

More visibility produces diminishing then negative returns. Seven organizational frameworks independently warn about this ceiling. Even active engagement has a throughput limit. Beyond it, developers either disengage or burn cognitive budget. The bounded read window (default 50) is a hypothesis about where this ceiling is.

### 4.5 Team Coherence vs. Developer Preference for Isolation

Every successful AI tool is single-player. 81% AI coding tool adoption, all of them single-player. Only 17% say AI improved collaboration. The market is solving context loss by making solo sessions smarter, not by connecting sessions. The gap argument ("no tool does this") could equally mean nobody wants it.

### 4.6 The Vulnerability Loop

Trust between humans forms through reciprocal vulnerability. When AI handles the vulnerable parts of collaboration (asking "dumb" questions, admitting uncertainty), it absorbs the vulnerability that should flow between humans. Result: rich human-AI trust, shallow human-to-human trust. This finding is theoretical and untested.

### 4.7 Scaffolding vs. Permanent Assistance

Every educational framework insists AI support must decrease as competence increases. The expertise reversal effect (d = -0.428): the same AI explanation that helps a junior creates measurable drag for a senior. One level of reasoning visibility cannot serve both. This implies bpmux would need adaptive display — added complexity for a v1 product.

---

## 5. What the Corpus Agrees On (High Confidence)

These findings are confirmed by multiple independent analyses with convergent evidence:

1. **The persistent shared log is the core primitive.** Architecturally novel (Wave 1), economically motivated by the review bottleneck (Wave 2), theoretically predicted to change team dynamics (Wave 3). The log is the product.

2. **bpmux must stay a communication layer.** Architectural analysis (Wave 1), market positioning (Wave 2), Bitter Lesson (Wave 3). Three independent paths, same conclusion. The AI lives outside.

3. **The isolation problem is real and documented.** GitClear, Faros AI, METR, JetBrains — independent sources, convergent findings. AI makes individuals faster and teams less coherent.

4. **Nothing has been validated with real usage.** Every wave ended at the same place: test before theorizing further.

---

## 6. What the Corpus Disagrees On or Cannot Resolve

1. **Whether developers want shared AI reasoning.** The evidence is mixed. Kurz (2025) found human+AI pairs outperform solo+AI. But the Nature meta-analysis found human-AI combinations underperform on decision tasks. The ICSE 2025 study found developers share AI conversations deliberately (to justify, explain, seek help), not casually — challenging the ambient-visibility model. 83% of developers did not report AI improving collaboration.

2. **Whether the transparency paradox is manageable or structural.** Bernstein's findings could be a design challenge (solvable with calibrated transparency) or a fatal flaw (visibility inherently harms the thing bpmux wants to create).

3. **Whether the "beyond pairing" reframe is principled or reactive.** The move from "pairing tool" to "shared environment" was triggered by MCP's technical limitations (can't do real-time). The research reframed this as an insight ("campfire metaphor"). The self-critique identifies it as convenience. It may also be correct. It is not yet clear which.

4. **Whether the Storey framework is load-bearing.** Cognitive debt and intent debt are central to bpmux's thesis. The framework is one month old and unvalidated. If these concepts do not hold up under empirical testing, the theoretical foundation weakens. The practical problem (reasoning evaporates) still exists regardless.

---

## 7. What Should Never Be Built

These survive criticism intact across all waves and reviews:

- An orchestrator or encoded coordination patterns (Bitter Lesson + Yegge's failed iterations)
- An embedded LLM client inside bpmux (four analyses converge)
- A surveillance tool (destroys the value it aims to create)
- Tool execution or coding capabilities inside the chat (bpmux is a communication layer)
- Sophisticated AI behavior before validating basic interaction (start with "respond when mentioned")
- Ambient visibility as default without resolving the transparency contradiction first

---

## 8. Hypotheses That Must Be True for bpmux to Work

From the adversarial synthesis. If any of these is false, the product has no reason to exist in its current form:

1. **Captured reasoning produces better outcomes than ephemeral reasoning.** The foundational claim. If the log is not useful, there is no product.

2. **Developers will actually write reasoning into the log.** If developers use bpmux like Slack (terse) rather than like a decision journal (reasoning-rich), intent debt prevention fails.

3. **The value of shared context exceeds the cost of producing and consuming it.** Token costs, attention costs, and coordination overhead must be net positive.

4. **A communication layer without AI integration is enough.** bpmux depends on external tools to make AI participation work. If MCP polling is too slow and push never materializes, bpmux is a chat room nobody talks in.

5. **The problem will not be solved by the platforms themselves.** Claude Code, Cursor, or Copilot could ship shared sessions as a feature. Market risk with a clock on it.

---

## 9. What This Distillation Gets Wrong

This section exists because distillation is inherently a selection process, and selection is confirmation bias with better manners.

**Cherry-picking.** I chose which findings to emphasize. The narrative I find compelling (the problem is real, the primitive is novel) shapes which evidence appears prominently and which gets a sentence in a caveat. A reader should treat my emphasis as editorial choice, not as evidence weighting.

**Confidence inflation.** The previous distillation called design hypotheses "proven." This version corrects that. But I may still be inflating confidence in subtler ways — through word choice, through what I place first, through what I present as "the corpus agrees" versus "the corpus disagrees."

**The organizational frameworks and analogies.** I've labeled them decorative and moved on. But I may be underweighting them. Conway's Law does generate testable predictions about bpmux teams. The jazz improvisation analogy (working agreement as harmonic vocabulary) does produce a non-obvious design insight. My correction may overcorrect.

**Absence of user perspective.** Zero users have used bpmux for its intended purpose. Every claim about what developers want, how they will behave, and whether the tool is useful is theory. The corpus is thorough, well-sourced, and validated against nothing.
