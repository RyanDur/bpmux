# Arc Prior Art Research

Date: 2026-04-07
Source: PM research agent, cross-referencing existing corpus

## Summary

Arcs — reasoning paths that capture how a team arrived at understanding — have deep roots across multiple academic fields. This document maps those connections and extracts practical lessons for building arc capabilities into bpmux.

## Three Cross-Cutting Themes

1. **The middle layer is the universal gap.** Every field identified the same problem: raw experience is too noisy, abstracted rules are too thin, and the reasoning path between them is what transfers understanding.
2. **Capture cost determines survival.** Systems requiring dedicated effort to capture reasoning failed. Systems capturing it as a side effect of work succeeded.
3. **Walking beats reading.** Understanding transfers through walking reasoning paths, not reading conclusions.

## 1. Case-Based Reasoning (CBR)

**Key researchers:** Roger Schank, Janet Kolodner, Agnar Aamodt, Enric Plaza

An arc IS a case in CBR terms. Schank's *Dynamic Memory* (1982) argued memory is organized around experiences, not abstract rules. MOPs (Memory Organization Packets) organize scripts into hierarchies for cross-domain transfer.

**CBR cycle** (Aamodt & Plaza, 1994): Retrieve similar case → Reuse solution → Revise for current problem → Retain new case.

**Connection to arcs:** Arc "turns" correspond to Schank's expectation failures — moments where the old script broke. The many-to-many relationship between arcs and lessons maps to CBR's indexing structure. The weighting scheme (frequency, recency, source, reversal) is standard CBR retrieval ranking.

**Key lesson:** The hardest part of any case library is **indexing** — not storage but findability. Arcs should be indexed at multiple levels of abstraction simultaneously.

**Caution:** CBR worked well in narrow domains, poorly in broad ones. A team's arc library is narrow — that's favorable.

## 2. Nonaka's SECI Model

**Key researchers:** Ikujiro Nonaka, Hirotaka Takeuchi, Noboru Konno

The concept of **ba** (shared context for knowledge creation) is the key connector. The bpmux chat during a correction sequence IS originating ba — shared context where tacit understanding transfers through interaction.

**Connection to arcs:** Arcs preserve the ba. Not the knowledge itself (that's a lesson/document), but the shared context in which knowledge formed. Externalization (tacit→explicit) typically loses conversational scaffolding. Arcs preserve the externalization *process* — enabling re-internalization because context is intact.

**Key lesson:** Preserve the dialoguing ba — peer-to-peer conversations where tacit knowledge first becomes articulable. Owner's Socratic questions are dialoguing ba in action.

**Caution:** Nonaka's model is descriptive, not prescriptive (Gourlay, 2006). Tells you what conversion looks like, not how to reliably produce it.

## 3. Naur's "Programming as Theory Building" (1985)

**Key researcher:** Peter Naur

Theory = mental model of how program relates to real-world affairs. Theory cannot be communicated through documentation. Can only be rebuilt through **walkthrough** — having original developers explain thinking, answer questions, work through reasoning together.

**Connection to arcs:** Arcs are **asynchronous walkthroughs**. They preserve enough reasoning sequence that a new participant can rebuild theory without original participants present. This is "borrowed nurture" — the strongest theoretical justification for arcs.

**Key lesson:** The correspondence is precise: Naur's theory includes (a) problem-solution relationship, (b) why the structure is what it is, (c) how it can be modified. A well-captured arc contains all three.

**Caution:** Even walkthroughs are imperfect. The receiver builds their *own* theory, which may differ from the original. Arcs accelerate theory-building but can't guarantee identical theory.

## 4. Szulanski's Knowledge Transfer Barriers

**Key researcher:** Gabriel Szulanski

"Exploring Internal Stickiness" (1996): Biggest barriers to knowledge transfer are NOT motivational. They are:
1. **Causal ambiguity** — source doesn't understand why their practice works
2. **Absorptive capacity** — recipient lacks background to understand
3. **Arduous relationship** — poor communication channel
4. **Ramp-up failure** — transfer fails during practice, not handoff

**How arcs address each:**
- Causal ambiguity: arcs preserve the causal chain explicitly
- Absorptive capacity: arcs build the background within themselves — the sequence provides context needed to understand the conclusion
- Arduous relationship: arcs bypass direct source-recipient relationship entirely
- Ramp-up: arcs contain the unexpected problems (turns, reversals) that recipients need during ramp-up

**Caution:** Arcs help ramp-up but don't eliminate it. Practice is still required.

## 5. Argyris & Schon's Single/Double-Loop Learning

**Key researchers:** Chris Argyris, Donald Schon

**Single-loop:** correct the action without questioning the framework.
**Double-loop:** question the governing variable and change it.

**Connection to arcs:** The most valuable arcs are double-loop — they capture assumption changes, not just behavioral corrections. The handoff arc is textbook double-loop: the team's governing variable changed from "transfer knowledge through documents" to "the log IS the teaching medium."

**Key lesson:** Arcs containing owner questions are disproportionately likely to be double-loop and should be weighted accordingly. Socratic questioning forces assumption examination.

## 6. Design Rationale Systems

**Key researchers:** Kunz & Rittel (IBIS), MacLean et al. (QOC), Lee (DRL), Conklin (gIBIS)

**Why most failed:**
1. Capture cost: 2-3x longer than making the decision
2. Wrong granularity: forced structure on unstructured reasoning
3. No immediate payoff: benefits future readers, costs current writer
4. Staleness: rationale becomes outdated as code evolves
5. Retrieval failure: structured representations were hard to search

**What arcs learn from DR failures:**
- Must be zero or near-zero capture cost (#1 lesson)
- Don't impose structure on reasoning — arcs are message sequences, not structured tuples
- Must solve the immediate payoff problem — Friday migration tests whether arcs produce visible value
- Arcs age better than rationale because they capture reasoning patterns, not specific decisions

**Most successful DR approach:** Carroll's claims analysis — lightweight, states claims with tradeoffs rather than full argument networks.

## 7. Experience Factories

**Key researchers:** Victor Basili, Gianluigi Caldiera, H. Dieter Rombach

NASA/SEL Experience Factory ran 20+ years. Defect rates dropped 75%. One of the few KM initiatives that demonstrably worked.

**Why it succeeded:**
1. Organizational commitment (decades, not quarters)
2. Dedicated packaging role (separate organizational unit)
3. Tight feedback loop (value visible within next project)
4. Quantitative grounding (everything measured)

**Connection to arcs:** Experience packages sit between raw data and abstract guidelines — exactly where arcs sit. An AI that detects and packages arcs from the log could serve the "experience factory" role.

## 8. Apprenticeship / Situated Learning

**Key researchers:** Lave & Wenger, Collins, Brown & Duguid

Walking an arc is **compressed cognitive apprenticeship** (Collins et al., 1989):
- Modeling: arc shows expert reasoning in action
- Articulation (inverted): original participants articulate; walker reconstructs
- Reflection: walker compares intuitions to arc's reasoning

**Missing:** Coaching and scaffolding require a responsive expert. An arc is a recording, not a tutor.

**Key insight:** Brown & Duguid (1991) argued that canonical practices (docs) and non-canonical practices (reality) always diverge. Arcs capture non-canonical practice — exactly what newcomers need and can't get from documentation.

## 9. Systems That Actually Succeeded

| System | Why it worked |
|--------|--------------|
| NASA/SEL Experience Factory | Dedicated staff, decades-long commitment, quantitative grounding |
| Toyota A3 Reports | Short, structured, embedded in daily practice, captures reasoning path |
| Stack Overflow | Zero capture cost for asker, immediate payoff, archive is side effect |
| Blameless postmortems (Google SRE, Etsy) | Embedded in workflow, immediate value, timeline preserves sequence |
| Pattern Languages (Alexander, GoF) | Indexed by problem, carry consequences, communally developed |

**Common success traits:**
1. Capture is a side effect of work
2. Immediate payoff for the capturer
3. Retrieval at the moment of need
4. Format preserves sequence/reasoning
5. Social infrastructure keeps knowledge alive

## 10. Nature vs Nurture in AI

**Base model = nature** (pretraining). **Arcs = nurture** (in-context learning).

**Key findings:**
- ICL is pattern-sensitive: arc *shape* (question→discovery→change) may teach as much as content (Min et al., 2022)
- Chain-of-thought prompting: arcs ARE reasoning demonstrations. Loading arcs into context is literally giving chain-of-thought examples about team reasoning (Wei et al., 2022)
- ICL is context-window-limited: arcs must be concise. Critical arcs may need distillation into more durable forms
- System prompt sensitivity: arcs CAN shape agent behavior, but conflicting/manipulated arcs could mislead

**Practical implication:** Current models have sufficient "nature" to benefit from arc-based "nurture." The 80-message log experiment (4.5/5 culture score) is evidence.

## Implications for bpmux

1. **Don't build IBIS.** Design rationale systems failed. Arcs succeed only if capture is near-zero cost.
2. **The log is the raw material.** Arcs are pointers into existing conversation, not new artifacts.
3. **Index by the type of misunderstanding corrected**, not just topic keywords.
4. **Owner corrections are highest-weight signals** — they're the expectation failures that drive learning.
5. **Process creates clean arcs naturally** — retros, acceptance, "one voice," Socratic questioning all produce better arc material.
6. **An AI can serve the Experience Factory role** — detecting, packaging, and surfacing arcs without human curation burden.
7. **Walking the arc is the product experience** — not reading a summary, but following the reasoning path.
