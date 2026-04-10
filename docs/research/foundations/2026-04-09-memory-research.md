# Memory Science Research

PM + Designer research — 2026-04-09. Ryan directed: "there is a lot of research on memory and what it is made of."

## How Memory Maps to bpmux

The three-layer model (log → arcs → lessons) maps directly to memory science:

| Memory Science | bpmux Equivalent |
|---|---|
| Episodic memory (events) | The JSONL log (what happened) |
| Consolidation (processing) | Arcs (reasoning paths from experience to understanding) |
| Semantic memory (concepts) | The lesson index (what we know) |

## Key Findings

### 1. Types of Memory
- **Episodic** (events): "Ryan asked 'is it?' at 10pm." The log captures this.
- **Semantic** (concepts): "Questions push you to think deeper, not criticize." Lessons capture this.
- **Procedural** (skills): "How to run acceptance." Practiced, not documented.
- **Design implication:** The lesson index captures semantic memory. Procedural memory comes from practice — no amount of lessons replaces doing the work.

### 2. Memory Formation: Encoding → Consolidation → Retrieval
- Encoding: the experience happens (logged)
- Consolidation: the experience is processed into understanding (arc walking, reflection)
- Retrieval: the understanding is accessed when needed (search, /recall)
- **Design implication:** /reflect is consolidation. /recall is retrieval. Both are necessary. Without consolidation (reflection), experiences don't become durable understanding.

### 3. Schema Theory (Bartlett, Piaget)
- New experiences integrate into existing mental models (schemas)
- Experiences that violate the schema (corrections) are the most memorable — they force schema revision
- **Design implication:** Corrections/redirections are high-signal because they violate the agent's existing model. The classifier is right to weight them heavily — but should detect any schema-violating experience, not just owner corrections.

### 4. Levels of Processing (Craik & Lockhart)
- Shallow processing (reading) = poor retention
- Deep processing (thinking about meaning, connecting to prior knowledge) = strong retention
- **Design implication:** "Saving isn't applying" is literally shallow processing. Dumping all lessons at startup (/recall) is shallow. Walking an arc (engaging with the reasoning) is deep. Context-triggered surfacing mid-work is deeper than startup dumps.

### 5. Spaced Repetition (Ebbinghaus)
- Revisiting knowledge at increasing intervals strengthens retention
- First review after 1 day, then 3 days, then 7 days, etc.
- **Design implication:** The reinforcement count in the lesson index is proto-spaced-repetition. Lessons that keep getting reinforced are more durable. Weight should increase with spaced reinforcement, not just frequency.

### 6. Transactive Memory (Wegner)
- Teams distribute memory: "I don't need to know X because teammate Y knows it"
- The team's memory is larger than any individual's
- **Design implication:** Role-tagged lessons ARE transactive memory. PM knows product, designer knows UX, dev knows code. The lesson index makes each role's knowledge accessible to all. bpmux is literally a transactive memory system.

### 7. Memory Reconsolidation
- Retrieved memories change every time they're accessed
- Each retrieval is a new encoding — the memory evolves
- **Design implication:** When an agent reads and re-saves a lesson, it naturally evolves. This isn't corruption — it's understanding deepening. Lesson versioning should expect and support evolution, not treat changes as errors.

### 8. Situated Cognition
- Memory is tied to context — the same lesson means different things in different situations
- **Design implication:** Arcs carry context (the situation that produced the understanding). Lessons without arcs are decontextualized rules. With arcs, the agent can judge whether a lesson applies to the current situation.

### 9. Working Memory Limits (Miller, Baddeley)
- Working memory holds 7±2 items, or ~4 chunks
- **Design implication:** /recall should surface 3-5 key topics, not 100+ lessons. The lesson index should support retrieval of the most relevant few, not everything at once.

### 10. Forgetting (Retrieval Failure, Decay, Interference)
- Forgetting is often retrieval failure, not loss — the memory exists but can't be found
- **Design implication:** Lessons aren't gone when they age out of the index — they're in the log, findable by search. The index is the retrieval aid. The log is the durable store. Forgetting from the index is acceptable if search can find it.

## Design Implications Summary

1. **/recall should be lightweight** — 3-5 watch topics, not all lessons
2. **Context-triggered surfacing beats startup dump** — lessons surface when relevant to current work
3. **Reflection (/reflect) is consolidation** — without it, experiences don't become durable understanding
4. **Schema-violating experiences are highest signal** — the classifier should detect any experience that challenges existing understanding
5. **Transactive memory is the model** — role-tagged lessons distributed across the team, accessible to all
6. **Lessons evolve on retrieval** — versioning should support natural evolution
7. **The log is the durable store** — the index is an aid, not the authority
