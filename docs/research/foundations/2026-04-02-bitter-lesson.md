# The Bitter Lesson and bpmux

*Rich Sutton's "The Bitter Lesson" (2019) and what it means for our design.*

---

## The Argument

Sutton observed that across several landmark AI domains (chess, Go, speech recognition, computer vision) spanning decades, general methods that leverage computation consistently outperform domain-specific knowledge-based approaches. Building human knowledge into AI systems "always helps in the short term, and is personally satisfying to the researcher, but in the long run it plateaus and even inhibits further progress." The two methods that scale are search and learning — not human-engineered heuristics.

This is "bitter" because it means our hard-won expertise is less valuable than we want it to believe.

## Where It Challenges bpmux

### The working agreement encodes human team structures
Opus architects, Sonnet implements, Haiku verifies — these are human organizational patterns imposed on AI. Sutton's lesson suggests that encoding human team roles into AI coordination will eventually be outperformed by approaches that let AI learn its own coordination patterns. IC's "Clone Wars" article made the same argument explicitly, citing Sutton: swarms over hierarchies.

### Trust levels encode human security intuitions
Peer AI untrusted by default, local AI trusted — that's human judgment pre-encoded. A Sutton-aligned approach would let the system learn which participants produce reliable output based on track record, rather than assigning trust categories upfront.

### Message tiers encode human cognitive science
Focal/active/peripheral/ambient tiers (v1.x) apply human attention research to rendering decisions. Sutton might argue: let user behavior (what they expand, what they ignore) train the tier assignment rather than designing it from research.

## Where It Doesn't Apply

### bpmux is a tool for humans, not an AI system
Sutton's lesson is about building AI systems — chess engines, speech recognizers, Go players. These are systems where human knowledge is being encoded *into the AI's decision-making process*.

bpmux is not an AI system. It's a shared environment that humans and AIs operate in. The working agreement doesn't encode knowledge into Claude — it's a human process discipline that the humans follow. The trust levels don't train a model — they're a human security decision. The distinction matters: Sutton argues against baking human knowledge into AI. He doesn't argue against humans having processes for how they work with AI.

### The shared environment IS the general-purpose approach
Stigmergy — a shared log that any participant can read and write, with no centralized orchestration — is exactly the kind of simple, general-purpose mechanism Sutton's lesson favors. It doesn't encode specific coordination patterns into the system. It provides a medium through which patterns can emerge. The message log is to human-AI coordination what search is to chess: a general mechanism that scales, rather than a collection of hand-crafted heuristics.

### Human judgment is the "computation" in this system
Sutton says computation wins. In bpmux, the computation isn't FLOPS — it's human and AI reasoning applied to the shared environment. The more reasoning flows through the log, the richer the traces, the better the coordination. That's scaling through the shared environment, not through encoded knowledge.

## The Real Tension: v2 Orchestration

The Bitter Lesson bites hardest at the v2 roadmap. If v2 adds orchestration (Claude delegates to local models), we face a choice:

**Option A (knowledge-based):** Encode orchestration patterns — hierarchical delegation, role assignment, task decomposition rules. This is personally satisfying, immediately useful, and will eventually plateau.

**Option B (general-purpose):** Provide the shared environment and let coordination patterns emerge from usage. AIs that produce good results get more work (learned trust). Task decomposition emerges from conversation, not from encoded rules. This is harder to build, slower to show value, and — if Sutton is right — ultimately wins.

The spec already gestures at this: "both hierarchy and swarm emerge from the same stigmergic environment." That's the Sutton-compatible framing. But the working agreement is still a human-imposed structure, and v2 will be tempted to encode it into the tool.

**Our position:** The working agreement is a *human process*, not a *system design*. Humans can choose to work hierarchically or as a swarm. bpmux provides the environment; it doesn't impose the pattern. As long as v2 maintains this separation — general-purpose environment + human-chosen process — it's Sutton-compatible. The moment we hard-code orchestration logic into bpmux, we're on the wrong side of the Bitter Lesson.

## What This Means for the Design

1. **Keep bpmux general-purpose.** The message log, MCP tools, and P2P transport should not encode specific coordination patterns. They should enable any pattern.

2. **The working agreement is portable, not built-in.** It lives in AGENT_WORKFLOW.md, not in bpmux's code. Different teams can use different agreements. bpmux doesn't enforce one.

3. **Trust levels could eventually be learned.** v1 uses human-assigned trust categories (correct for now — we don't have enough data to learn from). A Sutton-aligned future version could supplement or replace these with track-record-based trust (participants who produce reliable output earn higher trust automatically).

4. **Message tiers could eventually be learned.** v1.x uses research-designed tiers (correct for now — we need sensible defaults). A Sutton-aligned future version could learn tier assignments from user behavior (what you expand = important, what you ignore = ambient).

5. **v2 orchestration should be emergent, not encoded.** Provide the environment for delegation and coordination. Don't hard-code the delegation patterns.

## The Synthesis

Sutton's lesson and Storey's triple debt model are complementary, not contradictory:

- **Storey** says: understanding is eroding. Capture intent. Maintain shared cognition.
- **Sutton** says: don't encode human knowledge into the system. Let general methods win.

bpmux threads this needle by being a **general-purpose shared environment** (Sutton-aligned) that **captures reasoning as a side effect** (Storey-aligned). The environment is simple and general. The understanding it preserves is emergent, not engineered.

The working agreement is a human discipline, not a system constraint. When the working agreement eventually evolves (as the spec says it will), bpmux doesn't need to change — because bpmux never encoded the agreement. It just carried the conversation.

## Sources

- [Rich Sutton — The Bitter Lesson (2019)](http://www.incompleteideas.net/IncIdeas/BitterLesson.html)
- [Bitter Lesson — Wikipedia](https://en.wikipedia.org/wiki/Bitter_lesson)
- [IC — The Eventual Clone Wars](https://initialcapacity.io/insights/eventual-clone-wars)
