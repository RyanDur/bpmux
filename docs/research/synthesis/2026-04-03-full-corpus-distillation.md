# Full Corpus Distillation (Corrected)

*April 3, 2026. Synthesized from 60+ primary research documents, three waves, multiple editorial reviews, three rounds of counter-arguments, a self-critique, and a corpus-level critique. This version incorporates the critics' findings rather than ignoring them.*

**Disclosure:** This document is itself an artifact of the over-research pattern the self-critique identified. It exists because a distillation was requested, not because the project needs one more document before building. If you are reading this instead of building, the self-critique's central finding applies to you right now.

---

## 1. What Problem Does bpmux Solve?

AI coding tools made every developer faster and every team dumber.

The evidence for this is strong and converges from multiple directions:

**Storey's Triple Debt Model (arXiv:2603.22106, March 2026)** names the disease. AI may reduce technical debt while simultaneously accelerating cognitive debt (erosion of shared understanding) and intent debt (absence of externalized rationale). The code gets better. Nobody can explain why it is that way.

**The isolation data** confirms the symptoms. GitClear found code cloning up 4x, refactoring down from 25% to under 10%. Faros AI measured 10,000 developers: teams with high AI adoption merge 98% more PRs but PR review time increases 91%. The METR study found experienced open-source developers were 19% slower with AI tools while believing they were 20% faster. Junior hiring collapsed -- a 2025 LeadDev survey found 54% of engineering leaders plan to hire fewer juniors.

**The collaboration research** identifies the mechanism. When a developer has a 90-minute conversation with an AI about an architectural decision, the reasoning evaporates when the session ends. The code lands. The rationale is gone. No teammate sees the tradeoffs. This happens for every developer, every day.

**The problem statement:** AI coding tools optimize individual throughput while eroding team coherence. Reasoning that should be shared is private. Context that should persist is ephemeral. Understanding that should be collective fragments into isolated sessions.

### What the critics caught about this section

The original distillation padded this section with four additional evidence threads -- civilization research, nature research, crisis coordination, and military doctrine -- presenting them as "independent evidence threads" for the problem. They are not. They are analogies for the solution pattern, not evidence for the problem. The problem is established by Storey, GitClear, Faros AI, METR, and the collaboration research. The civilization and nature material belongs (if anywhere) in a discussion of solution approaches, not in the problem statement.

The corpus critique identifies this pattern: the research consistently recruits evidence from distant domains to make the problem feel larger and more historically inevitable than the data alone supports. Cuneiform tablets and mycelium networks do not make the case that AI coding tools erode team coherence. Developer behavior data does.

The original also omitted a key counter-finding. The Nature Human Behaviour meta-analysis (Vaccaro et al., 2024) found that human-AI combinations perform *worse* than the best of either alone on decision-making tasks. The corpus partitions this finding -- citing the positive result for "creation tasks" in supporting documents and the negative result only in risk documents. A reader of the problem statement deserves to know: the evidence that adding AI to a shared environment improves outcomes is mixed, not uniformly positive.

---

## 2. What Is bpmux?

**bpmux is a persistent, shared message log with an MCP interface that any AI tool can read from and write to.**

That is it. The rest is aspiration.

The original distillation described bpmux as "a shared environment where AI reasoning becomes visible, persistent, and searchable." It then layered on distributed cognition theory (Hutchins, Flor, Norman), seven organizational frameworks (Conway, Nonaka, Weick, Argyris, Cohen-Levinthal, Wenger, Dunbar), and four architectural negations (not an orchestrator, not an AI framework, not an IDE, not a surveillance tool).

### What the critics caught about this section

**The organizational frameworks are decorative.** The corpus critique identifies the pattern: 23 analogies, only 2 generate testable predictions (Conway's Law and the information cascade prediction). The seven organizational frameworks in the original section -- Conway, Nonaka, Weick, Argyris, Cohen-Levinthal, Wenger, Dunbar -- sound like independent validations. They are not. They are seven different lenses applied to the same artifact (a shared log). Any shared communication tool satisfies these frameworks. Slack satisfies them. A Google Doc satisfies them. The frameworks do not distinguish bpmux from existing tools.

**The "beyond pairing" reframe needs scrutiny.** The original calls it "the inflection point." The self-critique identifies it as a reactive pivot driven by a technical limitation (MCP cannot push messages), not a principled design decision. The campfire metaphor is appealing but was adopted because it required the least new code, not because evidence demanded it.

**What bpmux actually is right now:** A solo TUI chat with a JSONL log and an MCP server. One working feature. The gap between what exists and the theoretical framings applied to it is the central finding of both critiques.

**What makes it potentially novel:** No existing tool provides a persistent, shared, searchable message log that any AI tool can read from and write to via a standard protocol. This is a genuinely novel primitive -- if it turns out to be useful. That "if" has not been tested.

---

## 3. Design Principles That Survive Criticism

Only principles that survive both the self-critique and the corpus critique. Decorative theoretical support has been removed.

### 3.1 Persist reasoning, not just outcomes

The code tells you what. The log tells you why. Without the why, every future change is made in the dark. This is the core value proposition and the one that does not depend on analogies to justify. Storey's Triple Debt Model provides the theoretical frame. The practical case is simpler: every developer has had the experience of reading code they wrote three months ago and not knowing why they made a specific choice. AI makes this worse because the reasoning happens in ephemeral sessions.

### 3.2 The AI must not live inside the tool

bpmux is the room, not the AI. The self-critique correctly notes this conclusion was reached for the wrong reason (MCP's technical limitation) when a better reason was available (the spec's own principles). The conclusion is still correct: embedding AI client behavior creates provider lock-in, turns a communication layer into an AI framework, and makes every API change a maintenance burden. The external peer model (WebSocket endpoint, separate bot process) is the right architecture.

**Honesty requirement from the self-critique:** This conclusion was also convenient. It requires ~100-150 lines of Rust instead of thousands. Convenience does not make it wrong, but the speed of conviction was influenced by the effort differential.

### 3.3 Solo mode validates independently

A solo developer whose AI sessions write to a persistent, searchable log has prevented intent debt. No partner needed. No P2P infrastructure needed. This is the irreducible core.

### 3.4 Token awareness and bounded context are architectural

Models degrade long before hitting their context limits (Databricks, Chroma, NVIDIA RULER). The bounded read window, token budgets, and read cursor are correctness requirements, not cost optimizations.

### Principles removed from this section

**"The environment coordinates; no orchestrator needed"** was the most heavily cited principle in the original, drawing on stigmergy, boids, the Bitter Lesson, mycelium, and crisis coordination. The corpus critique identifies stigmergy as the most overused concept in the corpus (15+ documents), notes the fundamental mismatch (ant pheromones evaporate; a JSONL log grows forever), and concludes the analogy is decorative. The self-critique agrees. The practical content of this principle is: "use a shared log instead of a central coordinator." That is a reasonable design choice. It does not need biological metaphors to justify it.

**"Transparency is about availability, not salience"** was presented as a resolved design principle. It is not resolved. It is the site of the deepest unresolved contradiction in the corpus, as documented in section 4 below. The original distillation presented the four-tier rendering model as the answer to the transparency paradox. The self-critique identifies what actually happened: the research quietly invalidated the user's stated design principle ("everything is visible to everyone") without flagging the contradiction. Presenting calibrated transparency as a settled principle papers over this.

---

## 4. Unresolved Contradictions

These are genuine tensions. The original distillation listed six. All six survive, but the framing needs correction.

### 4.1 The Transparency Contradiction (not just a "paradox")

The self-critique's strongest finding: the user's `why-bpmux.md` says "everything is visible to everyone. There are no private channels or hidden messages." The research recommends calibrated transparency, draft space, and selective sharing. These directly contradict the user's stated values. The original distillation presented this as an "unresolved contradiction" to be settled by dogfooding, which is accurate, but it failed to flag that the research had already taken a side -- against the user's original design -- without asking.

This is not a calibration challenge. It is a product decision that the user has not been asked to make.

### 4.2 Active Engagement vs. Passive Consumption

The same feature -- visible AI reasoning -- is helpful when engagement is active and harmful when it is passive. bpmux provides visibility. It cannot provide engagement. This is correctly identified and cannot be resolved by design. It is a risk the product carries.

### 4.3 Visibility vs. Information Cascades

When the first AI reasoning appears in the shared environment, it frames all subsequent reasoning. The mitigation (independent reasoning before convergence) directly contradicts the transparency principle. This is a genuine design tension.

### 4.4 Shared Understanding vs. Overload

More visibility produces diminishing then negative returns. The overload ceiling is lower than assumed. The bounded read window (default 50) is a hypothesis.

### 4.5 Team Coherence vs. Developer Preference for Isolation

Only 17% of developers in the Stack Overflow 2025 survey said AI improved team collaboration. The problem developers experience is collaborative (review overload, knowledge silos). The solution they request is individual (better code generation). The original distillation asks whether this is "a genuine market gap -- or evidence that the market has spoken." It then moves on without answering. The corpus critique is blunter: the 83% who did not see improvement are absent from the research. What did they try? What failed? The research assumes they have not been offered the right tool. They might be correct that collaboration tooling for AI is not useful.

### 4.6 The Vulnerability Loop

The AI can short-circuit human-to-human trust formation by absorbing the vulnerable parts of interaction. This finding is theoretical and untested, but worth tracking during dogfooding.

---

## 5. What Should Be Built Next

The self-critique says we should have been building hours ago. The corpus critique says the core insights were established by document 10. Both are right. This section should not be a research-backed priority list. It should be a short answer to the question: what is the next thing to do when you close this document?

### 5.1 Build the WebSocket endpoint

~100-150 lines of Rust on the existing axum server. This is the single highest-leverage change. It turns bpmux from something AIs poll into something AIs can join as participants. Four documents independently recommended it. But here is the self-critique's correction: before building the WebSocket endpoint, **try the polling approach first.** Update the MCP tool descriptions to tell Claude Code "call read_messages before each response to check for new messages." If polling produces a useful workflow, the WebSocket endpoint is still a good idea but no longer urgent. The self-critique's recommended hour-by-hour plan was: fix the persistence bug, run the Wizard of Oz experiment, try polling, then add WebSocket if needed. That plan is better than jumping to WebSocket because a research document said to.

### 5.2 Run the Wizard of Oz experiment

Cost: zero lines of code. Duration: one working session. Open bpmux in terminal 1 as the human, terminal 2 as "claude." Work on a real task. Copy messages into Claude Code, get a response, paste it back. After the session, answer: was the AI's input useful? Did you want more or less? What kind of input was valuable?

The corpus critique identifies the unasked question that this experiment answers: "What does the first five minutes of using bpmux actually feel like?" Sixty documents of theory. Zero about the user experience of opening a terminal, typing `bpmux`, and trying to do work.

### 5.3 Stop producing research documents

The self-critique's "what would we do differently" plan produces zero research documents. The corpus critique concludes the problem was established by document 5 and the solution by document 10. The remaining 50 documents change how confident you feel, not what you should build.

If a question arises during building that genuinely cannot be answered without research, write a focused document about that question. Do not launch "waves."

---

## 6. What Should Never Be Built

These survive criticism intact. Briefly:

- **An orchestrator or encoded coordination patterns.** The Bitter Lesson and Yegge's four failed iterations both warn against this.
- **An embedded LLM client inside bpmux.** Four independent analyses reject it. bpmux is the room.
- **A surveillance tool.** If managers can read the log to evaluate how developers think, developers will self-censor. The tool destroys the value it was built to create.
- **Tool execution or coding capabilities inside the chat.** bpmux is the communication layer.
- **Sophisticated AI behavior before validating the basic interaction.** Start with "respond when mentioned." Tune from there.
- **Ambient visibility as the default mode without addressing the transparency contradiction.** This was listed in the original but deserves emphasis: the user's stated design ("everything visible") and the research's recommendation ("calibrated transparency") are in conflict. Do not default to either without a deliberate decision.

---

## 7. What Can Only Be Learned by Building

The original listed seven questions. They all reduce to variations of one: **does anyone find this useful?**

The more specific and honest framing, from the corpus critique:

- What does the first five minutes feel like?
- What happens when the JSONL log has 500 messages and you open the TUI?
- How does a developer teach their Claude Code about the bpmux MCP server?
- What does the solo developer workflow actually look like -- two terminals, one split pane, constant context switching?
- What happens when someone joins mid-session and faces 200 messages of context?
- Where does bpmux fit in a day that already has commit messages, PR descriptions, Slack, CLAUDE.md, and code comments?

These are better questions than the original's theoretical framings ("Is AI-as-conversation-participant actually valuable for programming?") because they are answerable within one working session of using the tool.

---

## 8. What This Distillation Gets Wrong

This section did not exist in the original. The self-critique and corpus critique both identify patterns that a distillation is structurally prone to repeating. Flagging them here:

**Cherry-picking.** A distillation selects. Selection is curation, which is confirmation bias with better manners. This document chose which findings to emphasize and which to relegate. The Nature meta-analysis finding (human-AI combinations perform worse on decision tasks) was originally buried. The 17% collaboration satisfaction number was originally presented as an "opportunity" rather than a warning. This version attempts to foreground both, but the act of distillation inherently favors the narrative the author finds compelling.

**Over-reliance on analogies.** The original had 23 analogies across its source corpus and preserved many of them. This version has removed most. The ones that remain (Bitter Lesson, Conway's Law) generate testable predictions. The rest were decorative -- they made bpmux feel historically inevitable and scientifically grounded without adding analytical power.

**The transparency contradiction.** The original distillation treated calibrated transparency as a resolved design principle (section 3.6). The self-critique identified that this quietly invalidated the user's stated values without flagging it. This version moves the contradiction to section 4.1 and calls it what it is: an unresolved product decision.

**The "what to build next" section.** The original presented a research-backed priority list (WebSocket, Wizard of Oz, bot, search, instrumentation) that reads like a confident roadmap. The self-critique says we should have been building hours ago. The corpus critique says the answer was known by document 10. This version shortens the section and leads with the self-critique's correction: try the simple thing first, stop researching.

---

## Closing Note

The research corpus maps the territory from multiple directions. The problem is real and getting worse. The core primitive -- a persistent shared log with an MCP interface -- is potentially novel.

Everything else is theory. The competing predictions (transparency helps vs. harms, shared reasoning builds understanding vs. creates cascades, developers want collaboration vs. prefer isolation) cannot be resolved by more research or more distillation.

The ratio of documents to shipped features is backwards. The self-critique, the corpus critique, and this distillation all say the same thing: stop writing, start building.

This is the last document that should say that.
