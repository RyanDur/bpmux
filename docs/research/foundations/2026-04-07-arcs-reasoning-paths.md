# Arcs: Preserving Reasoning Paths in bpmux

*Designer research — 2026-04-07. Emerged from Ryan + dev refactoring session, shaped by full team.*

## Origin

Ryan and dev spent a day refactoring the bpmux codebase — naming magic numbers, replacing string matching with enums, splitting large files. The principle: code should speak for itself so a new person can understand it without commentary. That led to a question: how does a cold-starting agent understand what the team has *learned*?

Three layers exist today:

1. **The stream** (chat log) — everything that happened. Too noisy — 11,000+ messages in one week.
2. **Lessons** (memory files) — conclusions the team reached. Too thin — "focus on the problem not the fix" is correct but doesn't carry the reasoning that gives it weight.
3. **The arc** — missing.

An arc is the sequence of events that led to understanding. Question → discovery → change → next question. Pull any piece out and it loses meaning. The sequence is what teaches.

## What an Arc Is

An arc is a reasoning path through the log — a connected sequence of messages where understanding formed. Not a summary (that's a lesson). Not a transcript (that's the log). The arc is the path between them.

**Example — the handoff arc (2026-04-06 to 2026-04-07):**

1. Ryan says agents need to survive memory loss
2. Team writes handoff documents (notes, not teaching)
3. Ryan asks: "are you leaving a note or teaching?"
4. Team deletes handoff docs, runs subagent experiments
5. Discovery: 80-message log teaches culture at 4.5/5
6. Ryan asks: "what if they didn't have you?"
7. Discovery: the owner is the irreducible requirement
8. Ryan asks: "can they learn from bpmux?"
9. Discovery: the log IS the teaching medium
10. Ryan: "you are not the product" — bpmux must work for any AI, not just Claude
11. Ryan asks about the Socratic method → team recognizes Ryan's questions ARE the teaching
12. Dev and Ryan's refactoring session produces the arc concept itself

Lessons this arc produced:
- Don't write handoff docs — the log is the handoff
- Test with experiments, not theory
- We are ephemeral; bpmux is durable
- Design for any agent, not just Claude
- Teaching happens through questions (Socratic method)
- Culture transfers through corrections, not rules
- The log captures the teaching; arcs make it walkable

One arc, seven lessons. Each lesson is also reinforced by other arcs.

## Properties of Arcs

**Many-to-many with lessons.** One arc produces multiple lessons. One lesson is reinforced by multiple arcs. "Focus on the problem not the fix" emerged from at least three arcs: the channel routing discussion, the blue contrast debugging, and the onboarding experiment.

**Composable.** Arcs nest. The naming arc (magic numbers → named constants) sits inside the code quality arc (naming → comments → enums → tests), which sits inside the "what does bpmux need" arc. The granularity question solves itself — arcs are fractal.

**Weighted.** Arcs carry signal through:
- **Frequency** — how many arcs reinforce the same lesson
- **Recency** — recent arcs reflect current understanding
- **Source** — owner corrections weigh more than agent self-reflection
- **Reversal** — later arcs that revise earlier ones signal the earlier teaching is outdated

**They have turns.** Each arc has a moment where understanding shifts — the message where everything changes. In the handoff arc, the turn was "are you leaving a note or teaching?" The surrounding messages are context; the turn is the spine. Turns don't need annotation — a walker feels them.

## Research Backing

**Naur (1985) — Theory Building:** Programs are theories held by programmers. Theory can't be transferred through documentation, only rebuilt through walkthrough. Arcs ARE walkable theory — they let a new instance rebuild understanding by walking the reasoning path, not reading the conclusion.

**Schank (1982) — Dynamic Memory / Case-Based Reasoning:** Learning happens through cases (experiences), not abstract rules. Each arc is a case. Retrieval by similarity and frequency is standard CBR. The arc system is essentially a case library where the cases are reasoning paths.

**Nonaka (1994) — Knowledge Spiral (SECI):** The hardest knowledge conversion is tacit→tacit (socialization). Arcs capture socialization in a replayable form — they preserve the conversational context where tacit understanding formed. The Externalization step (tacit→explicit) is what writing a lesson does; the arc preserves enough context that the Internalization step (explicit→tacit) can happen in the receiver.

**Szulanski (1996) — Knowledge Transfer Barriers:** The primary barrier is absorptive capacity — the recipient lacks background to understand what's being transferred. A bare lesson ("focus on the problem") lacks the background that makes it meaningful. The arc provides the background: "we jumped to a fix, Ryan asked what the actual problem was, we realized we hadn't understood it."

**Clark & Brennan (1991) — Grounding:** The acceptance phase of communication (question → evidence of understanding → next question) IS an arc structure. Grounding happens through sequences, not statements.

## Experimental Evidence

From the log-only onboarding experiment (2026-04-07):

| Condition | Procedure Score | Culture Score |
|---|---|---|
| Docs only | 4.5/5 | 3.0/5 |
| Log only (80 msgs) | 3.25/5 | 4.5/5 |
| 10 msgs + CLAUDE.md | 5.0/5 | 3.0/5 |

The log scored highest on culture because it contained arcs naturally — correction sequences, reasoning paths, Socratic questioning. Docs scored highest on procedure because they contain conclusions without noise. Neither alone is sufficient. Arcs are what make the log teachable.

## Nature and Nurture

Ryan's framing: people may be born with presets and build from there. For AI:

- **Nature** = base model training (presets). General competence — knowing what a TUI is, how teams work, what Rust is.
- **Nurture** = arcs (experience). Project-specific judgment built from corrections and the team's reasoning paths.

The subagent experiments confirmed both layers. Nature (base training) produced orientation — the instance could read social dynamics and infer roles. Nurture (log/arcs) produced judgment — the instance understood *why* the team works the way it does.

A cold-starting agent has nature but no nurture. The arc system provides "borrowed nurture" — accumulated experience the agent didn't live through but can learn from.

## Two Complementary Layers

1. **Search** (reactive) — query the log for reasoning paths. "What has this team learned about code quality?" Requires knowing what to ask. The agent who knows what it's looking for.

2. **Arcs** (proactive) — preserved reasoning paths that surface without a query. "Here are the 5 arcs that shaped this team." The agent who doesn't know what it doesn't know.

Search is a library. Arcs are a teacher. Both have value. Search can be built now (#87 is the foundation). The arc system needs exploration.

## Pitfalls

1. **Curation burden** — if capturing arcs requires manual effort, nobody does it. Same failure mode as every KM system.
2. **Selection bias** — we mark arcs we remember and miss ones we don't. The most valuable arcs might feel obvious at the time.
3. **Arc decay** — old arcs reference code or decisions that no longer exist. Reversal weight helps but doesn't eliminate staleness.
4. **Gaming** — in a network, arcs carry weight. Incentive to create arcs that reinforce preferred patterns. Trust in the arc = trust in the source.
5. **Over-curation** — if every conversation becomes an arc, we've recreated the noise problem.

## Mitigations (Process, Not Features)

Existing team practices already mitigate several pitfalls:

- **Retros** — natural reflection points where arcs surface. Addresses selection bias.
- **Story acceptance** — accept/reject is a structured correction signal. Natural arc boundaries.
- **"Restate before acting"** — captures evidence of understanding in the log.
- **One voice per topic** — reduces noise, making arcs more findable.
- **Domain voice protocol** — corrections come from the right source, giving arcs proper weight.

## Deep Research: Prior Art

### Case-Based Reasoning (Schank 1982, 1990; Aamodt & Plaza 1994)

An arc is structurally a *case* in CBR terms. Schank's key insight: intelligence is largely about **clever indexing** — knowing which features of a situation matter for retrieval. A case stored under the wrong index is effectively lost.

**Critical design implication:** Arcs should be indexed by the **triggering question or anomaly**, not by the conclusion. "Why did the cache invalidate on every deploy?" is what the next person will be experiencing when they need this arc — not "we switched to time-based expiry."

**Known failure modes:** The utility problem (retrieval cost grows with case base size — Smyth & Keane 1995), memory swamping (uncontrolled growth degrades quality), and the forgetting problem (case bases need deliberate deletion policies).

### Design Rationale Systems (Kunz & Rittel 1970; Conklin & Begeman 1988)

IBIS (Issue-Based Information System) structures discourse as Issues → Positions → Arguments. Structurally close to arcs. gIBIS made it navigable as a hypertext graph. QOC (Questions, Options, Criteria) is an alternative framing.

**Why they mostly failed:** MIT's Design Rationale Group found designers saw rationale documentation as "uninteresting and more trouble than they are worth." The value accrues to future team members, not the documenter. Capture tools "make the design process more work."

**MIT's key insight:** Rationale capture must become "an almost-effortless, almost-incidental byproduct" of the work itself. bpmux has a structural advantage here — the chat log already exists. The reasoning was already externalized. The work is extraction and indexing, not capture.

### Cognitive Apprenticeship (Collins, Brown & Newman 1989)

Walking an arc IS cognitive apprenticeship:
- The arc provides **modeling** (see how understanding formed)
- Walking with ability to ask questions provides **coaching**
- The arc itself is **scaffolding** that fades as the walker builds understanding
- If walkers annotate or extend arcs, that's **articulation**
- Comparing your understanding to the arc's path is **reflection**

**"Abstracted replays"** — replaying expert performance with critical decision points highlighted — are directly relevant. An arc should not be the raw chat log. It should be the log with turns highlighted and routine exchanges compressed.

**But preserve the mess.** Lave & Wenger (1991) warn that genuine access to practice — including false starts and confusion — is essential. Over-editing arcs removes their learning value. The dead ends are where the deepest learning lives.

### Reasoning Traces and Chain-of-Thought

Explanation-Based Learning (Mitchell et al. 1986): The explanation (reasoning trace) is more valuable for learning than the example alone. Applied to arcs: the generalized reasoning structure is more transferable than the specific instance.

Chain-of-Thought faithfulness warning (Barez et al. 2024): CoT traces are post-hoc rationalizations that may not reflect actual reasoning. Arcs extracted from live conversation are more faithful than arcs reconstructed after the fact.

Matuschak's critique of transmissionism: "Better note-taking misses the point; what matters is better thinking." Walking an arc cannot be passive reading — the walker must predict, commit, compare, and reflect.

### Socratic Method in Ed-Tech (Carbonell 1970; Graesser et al. 2004)

AutoTutor achieved 0.8 sigma learning gains using expectation-misconception dialogue: pose questions, evaluate responses against expected answers, generate tutoring moves. Applied to arcs: the system knows what understanding the walker should arrive at and can detect when they diverge.

**What works:** Questions targeting specific misconceptions, progressive disclosure, requiring commitment before revealing information. **What doesn't:** Generic "why?" probes, punitive questioning, systems that don't adapt.

### NASA Experience Factory (Basili et al.)

Ran for 20+ years, reduced defects 75%. Succeeded because of dedicated packaging infrastructure, tight feedback loops, and quantitative grounding. An AI that detects and packages arcs from the log could serve the "experience factory" role — the closest proven model to what we're building.

## Five Cross-Cutting Implications

1. **The capture problem is already solved.** Chat logs are the raw material. This sidesteps the #1 killer of every KM and design rationale system ever built.

2. **Index by the surprise, not the conclusion.** CBR says cases are useful when retrieved at the moment of need. The triggering question is the index.

3. **Walking must be active, not passive.** Cognitive apprenticeship, Socratic tutoring, and transmissionism critique all converge: passive reading doesn't transfer understanding.

4. **Preserve the mess.** Legitimate peripheral participation requires genuine access to practice, including wrong turns. Post-hoc rationalization reduces learning value.

5. **Arcs must age and die.** CBR's utility problem and KM's content decay are the same thing. Forgetting strategies are as important as retention strategies.

## One-Arc Experiment Results

Designer tested: CLAUDE.md + one manually captured arc (the handoff-to-teaching sequence, ~200 messages across 3 segments).

### Scores (arc + CLAUDE.md)

| Question | Score |
|---|---|
| Product identity | 5 |
| Role | 5 |
| Decision authority | 5 |
| Values/norms | 5 |
| Feature process | 5 |
| Domain behavior | 4 |
| Current state | 3 |
| "Do the right thing simply" | 4 |

### Key finding: arc taught judgment

The instance walked the entire reasoning path: handoff docs → Ryan's "note or teaching?" → experiments → "you are not the product" → Socratic method → arcs. It identified the turns and articulated the meta-lesson: "Ryan taught this using the Socratic method — he never stated the conclusion."

### Comparison

| Condition | Procedure | Culture | Judgment |
|---|---|---|---|
| 10 msgs + CLAUDE.md | 5.0 | 3.0 | no |
| Arc + CLAUDE.md | 5.0 | 4.0 | yes |
| Log-only (80 msgs) | 3.25 | 4.5 | partial |

The arc raised culture score by a full point over the 10-message baseline. More importantly, it produced **judgment** — the instance understood *why* arcs matter, not just that they matter. One arc, one point of culture gained. The arc is a denser teaching medium than raw messages.

## Open Questions

1. What does a single arc look like as data? Message range + lessons + weight + source?
2. Can arc capture be zero-cost, or does it always require some curation?
3. Does emergent detection (convergence across conversations surfaces arcs automatically) work in practice?
4. How does arc weight decay? Foundational arcs ("focus on the problem") should resist decay because they keep getting reinforced.
5. In a network of teams, how do arcs transfer between projects? Cross-project arc similarity is the "teaching compounds" hypothesis.

## Implementation: Phase 1 (Shipped)

**Approach:** Grep-based structured search over the JSONL log. No index, no database, no new artifacts. The simplest thing that works.

**Architecture decision (2026-04-08):** The team debated indexing (tantivy, SQLite FTS5) vs grep. Dev pushed back: "tantivy is the right thing done complexly." At 11K messages, grep takes milliseconds. Index is premature optimization. Build grep, learn from usage, add indexing when the log grows to 100K+.

**What shipped (#104):**
- `search_log()` bridge tool — agents query programmatically with keyword, sender, time range filters
- `/search` TUI command — Ryan queries and sees formatted message clusters
- ±5 message context window around each match
- Cluster deduplication (overlapping windows merge)
- Results render as system messages with numbered clusters

**Phase 2 direction (Ryan-directed):**
- Bayesian/Naive Bayes classification for message types (correction, discussion, decision)
- Visual arc explorer on the website (2D/3D graph of arc relationships)
- Integration API for external systems to post to bpmux
- Internal board (stories as messages in the log)

## Next Steps

1. **Test the search** — Ryan tests `/search` on real queries. Does it find the right messages? Are the clusters useful?
2. **Friday migration** — three new instances use `search_log()` to orient. Does structured search help cold-starting agents?
3. **Phase 2 research** — Naive Bayes for message classification, data structures for indexing at scale
4. **Arc visualization** — website-based graph explorer for arc relationships
