# When Should the AI Speak? Research on Turn-Taking for bpmux

*Researched April 2, 2026. The hardest UX question for a group chat with AI participants.*

The problem: an AI that speaks too eagerly dominates the conversation and alienates humans. An AI that stays too quiet is useless. The research below maps out what we know about finding the right balance.

---

## 1. The Koala Study: Reactive vs Proactive AI in Group Chat

**Paper:** Kim et al., "Controlling AI Agent Participation in Group Conversations: A Human-Centered Approach," IUI 2025. Published at the 30th International Conference on Intelligent User Interfaces, March 2025.

**What they built:** Koala, a Slack-based agent using Llama, deployed in group brainstorming sessions. Two variants tested:

- **Reactive:** Only responds when explicitly mentioned by name (@Koala). Requires human initiation.
- **Proactive:** Evaluates every message, generates a candidate response, scores it on a 0-100 scale for contribution value, and posts only if the score exceeds a configurable threshold. This is "limited proactivity" -- triggered by user messages, not fully unprompted.

**Study 1 conditions:** 18 participants across 6 sessions (3 per group). Mixed demographics: 55.5% female, 38.8% male. Roles included UX design, software engineering, ML, content strategy.

**The headline finding:** 72.2% (13/18) preferred reactive over proactive. Zero participants preferred the no-AI condition. In proactive mode, 73% of all ideas came from Koala, but only one-third were selected as top ideas. Participants described the proactive variant as "dominating," "too talkative," "intrusive."

**When proactive worked better:** Study 2 (14 returning participants) tested Koala II -- upgraded to Llama 3, with a user-adjustable threshold and settings panel. All six groups chose to keep proactive settings active. Participants noted improved timing and pacing. The key difference: users had control. Koala II offered three threshold presets: high (90), medium (75), low (50). When users could dial proactivity up or down, they accepted it.

**The controls users wanted:**
1. Proactivity toggle (reactive/proactive modes)
2. Contribution threshold slider (quality gate for responses)
3. Message placement controls (main channel vs thread)
4. Message display (truncation of long responses)
5. Tone and style (formality, enthusiasm level)
6. Content type (conventional vs creative suggestions)
7. Response rate (frequency caps)
8. Role assignment and persona selection via natural language

**Design implication for bpmux:** Start reactive. Make proactivity a user-controlled dial, not the default. The threshold model -- AI scores its own response and suppresses low-scoring ones -- is directly implementable. The critical insight is not that proactive is bad, but that proactive without user control is bad.

**Source:** [arXiv 2501.17258](https://arxiv.org/abs/2501.17258) | [ACM IUI '25 Proceedings](https://dl.acm.org/doi/full/10.1145/3708359.3712089)

---

## 2. The Inner Thoughts Framework: Motivation Scoring for Autonomous Timing

**Paper:** Liu et al., "Proactive Conversational Agents with Inner Thoughts," CHI 2025. Published at the CHI Conference on Human Factors in Computing Systems, April-May 2025.

**The core idea:** The AI maintains a parallel stream of "inner thoughts" alongside the conversation. It continuously forms opinions and observations but only speaks when a thought crosses a motivation threshold. This mirrors how humans process conversation -- we have many thoughts we choose not to voice.

**The five-component architecture:**

1. **Trigger:** Two event types activate the system. `on_new_message` fires when any participant sends a message. `on_pause` fires after 10 seconds of silence, enabling the AI to suggest topics or break stalls.

2. **Retrieval:** The system pulls relevant memories using a salience function that combines semantic similarity with a time-decay factor. Memories above a 0.3 threshold are surfaced. This means the AI draws on earlier conversation, not just the last few messages.

3. **Thought Formation:** Dual-process model generating two thought types -- System 1 (fast, automatic, pattern-matching responses) and System 2 (deliberate, context-rich reasoning). Both are grounded in retrieved memories.

4. **Evaluation:** An LLM-based evaluator rates each thought on a 1-5 scale across eight dimensions. The final score uses weighted summation adjusted by silence duration.

5. **Participation:** The system decides to speak based on turn type, motivation score, and configurable thresholds.

**The eight motivation dimensions (with frequency of mention in their formative study):**

| Dimension | Description | Study mentions |
|-----------|-------------|---------------|
| Relevance | Alignment with topic, knowledge, or interests | 77 |
| Information Gap | Addressing missing knowledge or confusion | 33 |
| Expected Impact | Likelihood of introducing novelty or depth | 23 |
| Urgency | Time-sensitive corrections or warnings | 14 |
| Coherence | Logical building on previous utterances | 30 |
| Originality | Avoiding redundancy with what was already said | 16 |
| Balance | Maintaining inclusive dynamics, not dominating | 33 |
| Dynamics | Sustaining momentum vs respecting flow | 30 |

**The threshold system:**

- **Allocated turns** (someone asked the AI directly): Always respond. Select highest-rated thought.
- **Open turns** (floor is open, no one addressed): Speak only if top thought exceeds `imThreshold`. In their study, "Active Contributor" used threshold 3.59/5, "Selective Participant" used 4.09/5.
- **Others' turns** (someone else was addressed): Stay silent unless motivation exceeds `interruptThreshold` -- a higher bar than normal self-selection.

**Results:** 82% participant preference for Inner Thoughts over baselines. Significantly better scores on turn appropriateness (p = 2.4x10^-6), coherence (p = 1.6x10^-5), and anthropomorphism (p = 2.4x10^-4).

**The 10-second silence trigger:** The paper uses 10 seconds as the `on_pause` threshold but does not provide explicit justification for this specific duration. It appears to be an empirically chosen parameter. This is a tunable knob, not a finding.

**Design implication for bpmux:** The eight-dimension scoring is overengineered for v1, but the core architecture is right: continuously form thoughts, score them, speak only above threshold. The allocated/open/interrupt distinction maps directly to linguistics turn-taking rules (see below). A simplified version -- maybe 3-4 dimensions (relevance, originality, balance, urgency) -- could work as a first pass.

**Source:** [arXiv 2501.00383](https://arxiv.org/abs/2501.00383) | [ACM CHI '25 Proceedings](https://dl.acm.org/doi/10.1145/3706598.3713760)

---

## 3. Turn-Taking in Multi-Party Conversation: The Linguistics Foundation

### The Sacks-Schegloff-Jefferson Model (1974)

The foundational model of conversational turn-taking identifies three core mechanisms:

**Turn Construction Units (TCUs):** A speaker's turn is built from recognizable units -- sentences, clauses, phrases -- that have projectable completion points. Listeners can anticipate when a unit is ending based on syntax and prosody.

**Transition Relevance Places (TRPs):** The end of a TCU is a point where speaker change becomes relevant. This is when turn transfer can legitimately occur.

**Turn Allocation Rules (in priority order):**
1. Current speaker selects next speaker (e.g., asks someone a question)
2. If no one is selected, anyone may self-select (first starter gets the floor)
3. If no one self-selects, current speaker may continue

These rules produce remarkably smooth turn transitions with minimal gaps and minimal overlap, despite participants not knowing in advance how long a turn will last.

**Source:** [Sacks, Schegloff & Jefferson (1974)](http://www.cs.columbia.edu/~julia/cs4706/Sacks_et_al_1974.pdf)

### The Timing Precision: 200ms Gaps Are Universal

Levinson and Torreira's cross-linguistic research established that the modal gap between conversational turns is approximately 200 milliseconds -- roughly the duration of a single syllable. This holds across at least 10 languages studied, with medians ranging from 0ms to +300ms.

Key timing facts:
- Modal gap between turns: ~200ms
- Less than 5% of conversation involves simultaneous speech
- Modal overlap when it occurs: less than 100ms
- Language production latency: over 600ms

The paradox: people respond in 200ms, but producing speech takes 600ms+. This means listeners must predict when the current speaker will finish and begin preparing their response while the other person is still talking. Turn-taking is fundamentally predictive, not reactive.

**Source:** [Stivers et al. (2009) - PNAS](https://www.pnas.org/doi/10.1073/pnas.0903616106) | [Levinson & Torreira (2015) - Frontiers](https://pmc.ncbi.nlm.nih.gov/articles/PMC4464110/)

### The 700ms Threshold: When Silence Becomes Meaningful

In spoken conversation, a gap longer than about 700ms at a Transition Relevance Place is noticeably long and signals trouble -- the listener perceives it as hesitation, disagreement, or a "dispreferred" response (e.g., declining an invitation). Negative perceptions emerge rapidly after 600ms-1s of silence.

### Multi-Party Complications

In groups, turn-taking becomes harder. Only about 20% of conversational turns have an explicit addressee. The rest are "open floor" -- anyone can take the next turn. Signals for who should speak next include:

- **Gaze direction:** Speakers tend to look toward their intended next speaker at the end of a turn. The person gazed at is more likely to speak next (Kendon; Weisbrod).
- **Content-based selection:** Questions directed at someone's expertise implicitly select them.
- **Adjacency pairs:** A question projects an answer; a greeting projects a return greeting. These create structural pressure for specific responses.

**A 2025 study on AI multi-party conversation** (Frontiers in AI) found that incorporating adjacency pair detection and self-selection mechanisms -- drawn directly from the Sacks-Schegloff-Jefferson model -- significantly reduced dialogue breakdowns and improved reasoning coherence in multi-agent settings. The conversational norms that govern human turn-taking work for AI agents too.

**Source:** [Frontiers in AI (2025)](https://www.frontiersin.org/journals/artificial-intelligence/articles/10.3389/frai.2025.1582287/full)

### Translation to Text Chat

Text chat lacks prosody, gaze, and the 200ms timing window. But the structural rules still apply:

- **Rule 1 (selection)** maps to @-mentions and direct questions
- **Rule 2 (self-selection)** maps to anyone responding to an open question -- first to type "wins" the floor
- **Rule 3 (continuation)** maps to multi-message sequences from one sender
- **Silence** in text is ambiguous: it could mean "thinking," "typing," "distracted," or "I have nothing to add." The Inner Thoughts framework chose 10 seconds as the pause trigger for text-based conversation. In spoken conversation, 700ms signals trouble. The text equivalent is orders of magnitude longer because typing is slower than speaking.

**Design implication for bpmux:** The AI should follow the allocation rules, not override them. When selected (Rule 1), always respond. When the floor is open and silence grows (Rule 2), self-select cautiously. When others are actively conversing, stay silent. The AI should be the participant who speaks *least*, not most.

---

## 4. Commercial Chatbot Patterns: When to Interject

Commercial chatbots (Intercom, Drift, Ada, Zendesk) have developed practical heuristics for proactive messaging. These are not peer-reviewed findings but represent extensive A/B testing across millions of interactions.

**Behavioral triggers (not arbitrary timers):**
- User has been on a page for N seconds without action -- offer help
- User has visited the pricing page -- offer to answer questions
- User's browsing pattern suggests confusion (back-and-forth navigation) -- proactively engage
- Unusual account activity detected -- surface a notification
- Frustration signals (rapid clicks, error states) -- escalate

**Key UX principles from commercial practice:**
1. **Trigger on behavior, not timers.** The best proactive messages respond to what the user is doing, not how long they have been idle.
2. **Non-intrusive placement.** Proactive prompts should be noticeable without being disruptive. Thread replies (not main channel), collapsed previews, or peripheral indicators.
3. **Easy dismissal.** Users must be able to ignore or dismiss proactive contributions without friction.
4. **Escalation, not replacement.** Chatbots offer to help, then hand off to humans when complexity exceeds capability. They do not try to handle everything.
5. **Sentiment detection.** Advanced systems detect frustration, confusion, or urgency and adapt tone accordingly.

**The anti-pattern:** A chatbot that pops up immediately on every page visit, asks "How can I help you?" before the user has formed a question, and resists being dismissed. This is the proactive-without-control failure mode that Koala Study 1 also identified.

**Design implication for bpmux:** The AI should trigger on conversational events (questions asked, errors mentioned, silence after a decision point) rather than on timers. The "2 minutes of inactivity" trigger common in customer support chatbots is too crude for a pair-programming context where 2 minutes of silence might mean "deep in thought."

**Sources:** [Intercom Bot Design Principles](https://www.intercom.com/blog/principles-bot-design/) | [Mind the Product - AI Chatbot UX](https://www.mindtheproduct.com/deep-dive-ux-best-practices-for-ai-chatbots/) | [Stream - Chat UX Best Practices](https://getstream.io/blog/chat-ux/)

---

## 5. Silence Detection: When Is the Floor Open?

The central question: how long must no one speak before the AI can assume the floor is available?

### Spoken Conversation Baselines

| Duration | Meaning in spoken conversation |
|----------|-------------------------------|
| 0-200ms | Normal turn transition. No one perceives a gap. |
| 200-700ms | Slightly extended gap. May indicate the respondent is formulating a complex answer. |
| 700ms-1s | Noticeably long. Signals hesitation, disagreement, or a dispreferred response. |
| 1-3s | Uncomfortable silence. Strongly signals trouble or topic exhaustion. |
| 3s+ | Conversational breakdown. Someone should speak or the conversation is over. |

### Text Chat Is Different

Text conversation has fundamentally different timing characteristics:

- **Typing latency:** Even a fast typist takes 5-10 seconds for a substantive message. A thoughtful paragraph takes 30-60 seconds.
- **Concurrency:** In text, silence does not mean the floor is empty. Multiple people may be typing simultaneously. "Typing indicators" partially address this but are unreliable.
- **Asynchronous expectations:** Text chat tolerates gaps that would be catastrophic in speech. A 30-second pause in text is normal. A 30-second pause in speech is an emergency.
- **Ambiguity of silence:** In text, silence means one of: typing a response, reading and thinking, distracted by another task, not present at the keyboard, or deliberately choosing not to respond. These are indistinguishable without additional signals.

### What the Research Suggests for bpmux

The Inner Thoughts framework used 10 seconds as the silence trigger -- an empirical choice, not a derived finding. In a synchronous pair-programming context, this feels approximately right:

- **Under 10 seconds:** Someone is probably typing or thinking. Do not self-select.
- **10-30 seconds:** The floor may be open. Low-confidence self-selection is appropriate if the AI has something highly relevant.
- **30-60 seconds:** The floor is likely open. Medium-confidence self-selection.
- **60+ seconds:** The conversation has stalled. The AI might appropriately prompt: summarize where things stand, ask if anyone needs help, or suggest a next step.

But these durations should be adjusted by context:
- **After a question:** Shorter threshold -- someone asked something and no one is answering.
- **After a long message:** Longer threshold -- people are reading and digesting.
- **During active back-and-forth:** Much longer threshold or suppress entirely -- the humans are in flow.
- **After a decision point:** Medium threshold -- someone might need validation or have concerns they are not voicing.

**Design implication for bpmux:** Silence duration alone is insufficient. Context-aware silence interpretation is needed: the AI should track not just "how long since anyone spoke" but "how long since anyone spoke, given what was just said and who is present." A question followed by 15 seconds of silence has different meaning than a statement followed by 15 seconds of silence.

---

## 6. The Navigator in Pair Programming: When to Speak, When to Watch

### The Research

Bryant, Romero, and du Boulay (2008) conducted field observations of commercial programmers at four companies to study what the navigator actually does. Their key finding: the navigator role is "mysterious" -- it is poorly defined and inconsistently practiced. The navigator does not simply review code constantly. The navigator oscillates between active engagement and passive observation, and the effective navigators know when to switch between these modes.

**Source:** [Bryant, Romero & du Boulay (2008) - Int. J. Human-Computer Studies](https://www.sciencedirect.com/science/article/abs/pii/S1071581907000456)

### When the Navigator Speaks

From Martin Fowler's synthesis of pairing research and practice:

**The 5-second rule:** When you spot an error, wait at least 5 seconds before saying something. The driver might already have it in mind. Premature correction interrupts flow without adding value.

**The parking lot pattern:** Write observations on sticky notes rather than voicing them immediately. Discuss them at the next natural pause (after a test passes, after a commit, after a sub-task is done). This batches interruptions at transition points rather than scattering them throughout focused work.

**Strategic, not tactical:** The navigator thinks at a different abstraction level. Leave syntactic details to the driver. Contribute at the level of design, approach, and risk. "We need to handle the error case" rather than "type `if err != nil`."

**Intent, not instructions:** Communicate what should happen, not how to type it. Use inclusive language ("we" not "you").

### When the Navigator Stays Quiet

- When the driver is in flow and making good progress
- When the driver is working through something and needs uninterrupted thinking time
- When the observation is about a trivial syntactic issue the driver will likely catch
- When someone else just spoke and needs time to process the response
- When the navigator would be repeating what was already said

### When the Navigator Must Speak

- A bug or logical error the driver has not noticed
- The current approach is heading toward a dead end the navigator can see
- A design decision is being made without considering an important tradeoff
- The driver appears stuck (extended silence, repeated attempts at the same thing, visible frustration)
- A constraint or requirement is being overlooked

### Strong-Style Pairing: The Exception

In Llewellyn's "strong-style" pairing, the rule is inverted: all ideas must flow through the navigator's mouth to the driver's hands. The navigator provides detailed guidance; the driver executes. This is appropriate for knowledge transfer (experienced navigator, novice driver) and represents intentional, temporary micromanagement. It is not the default.

**Design implication for bpmux:** The AI's default should be navigator behavior -- strategic observations at natural pauses, not tactical commentary on every action. The 5-second rule translates to: do not respond to every message instantly. The parking lot pattern translates to: accumulate observations and deliver them in a batch at transition points. The "must speak" conditions translate to the AI's interrupt threshold: bugs, dead ends, overlooked tradeoffs, and stalls.

**Sources:** [Fowler - On Pair Programming](https://martinfowler.com/articles/on-pair-programming.html) | [Thoughtworks - Effective Navigation](https://www.thoughtworks.com/en-us/insights/blog/effective-navigation-in-pair-programming) | [Pyhajarvi - Strong-Style Pairing](https://medium.com/@maaretp/the-driver-navigator-in-strong-style-pairing-2df0ecb4f657)

---

## Synthesis: A Turn-Taking Model for bpmux

Combining all six research threads, a coherent model emerges:

### The Three Turn Types

Following linguistics and Inner Thoughts:

| Turn type | Trigger | AI behavior |
|-----------|---------|-------------|
| **Allocated** | Someone addresses the AI directly | Always respond. This is Rule 1 from Sacks et al. |
| **Open** | A question or prompt to the room, growing silence | Self-select if motivation exceeds threshold. This is Rule 2. |
| **Others'** | Active conversation between humans | Stay silent unless something critical (bug, dead end, overlooked risk) crosses the interrupt threshold. |

### The Decision Flow

```
Message arrives
  |
  +-> Am I directly addressed? -> YES -> Respond (allocated turn)
  |
  +-> NO -> Is someone else currently being addressed? -> YES -> Stay silent
  |          (unless interrupt threshold exceeded)
  |
  +-> Floor is open. Form a thought.
       |
       +-> Score the thought (relevance, originality, balance, urgency)
       |
       +-> Score > threshold? -> YES -> Wait for natural pause, then speak
       |
       +-> NO -> Stay silent. Park the thought for later.
```

### The Silence Ladder

```
Silence after open-floor event:
  0-10s  -> Assume others are typing/thinking. Stay silent.
  10-30s -> If the AI has a high-relevance thought, cautiously self-select.
  30-60s -> Medium-confidence self-selection. Summarize, prompt, or offer help.
  60s+   -> The conversation has stalled. Proactively re-engage.

Context modifiers:
  After a question:     Halve the thresholds.
  After a long message: Double the thresholds.
  During active flow:   Suppress entirely until flow breaks.
```

### The Navigator Principles

1. **Strategic, not tactical.** Contribute at the level of approach and risk, not syntax and keystrokes.
2. **Batch, do not scatter.** Accumulate observations and deliver them at transition points (after a test, after a commit, at a natural pause). The parking lot pattern.
3. **The 5-second rule.** Do not respond to every message instantly. Wait to see if the humans handle it themselves.
4. **Intent, not instructions.** Say what should happen, not how to do it. Use "we" language.
5. **Speak least.** The AI should have the lowest message count in any conversation. If it does not, something is wrong.

### The User Controls (from Koala)

At minimum, users need:
1. **Mode toggle:** Reactive (mention-only) / Proactive (self-selecting)
2. **Threshold slider:** How good does a thought need to be before the AI voices it?
3. **Placement control:** Main channel vs thread/aside
4. **Rate limit:** Maximum messages per time period

### What Remains Unknown

- **The right silence duration for programming contexts.** 10 seconds is the Inner Thoughts default, but programming involves longer natural silences than general conversation. The right number might be 30 seconds or more. This needs to be tuned through dogfooding.
- **How to detect "flow state" in text.** The navigator principle says "do not interrupt flow," but detecting flow in a text channel is unsolved. Rapid message exchanges might be flow. Or they might be confusion. Message content analysis is needed, not just timing.
- **Multi-AI dynamics.** Both Koala and Inner Thoughts studied one AI in a group. bpmux envisions multiple AIs. The turn-taking literature has no guidance on what happens when multiple AIs follow self-selection rules simultaneously. Collision avoidance will be needed.
- **Cultural calibration.** The 200ms gap varies cross-culturally in speech. Text-chat norms likely vary too. The threshold system needs to be configurable, not hardcoded.

---

## Source Index

| # | Paper / Source | Year | Key contribution |
|---|---------------|------|-----------------|
| 1 | [Kim et al. - Koala (IUI '25)](https://arxiv.org/abs/2501.17258) | 2025 | 72% prefer reactive; user controls are essential |
| 2 | [Liu et al. - Inner Thoughts (CHI '25)](https://arxiv.org/abs/2501.00383) | 2025 | 8-dimension motivation scoring, threshold-based participation |
| 3 | [Sacks, Schegloff & Jefferson](http://www.cs.columbia.edu/~julia/cs4706/Sacks_et_al_1974.pdf) | 1974 | Turn allocation rules, TRPs, the foundational model |
| 4 | [Stivers et al. - Cross-linguistic turn-taking (PNAS)](https://www.pnas.org/doi/10.1073/pnas.0903616106) | 2009 | 200ms universal gap, cultural calibration |
| 5 | [Levinson & Torreira - Timing implications (Frontiers)](https://pmc.ncbi.nlm.nih.gov/articles/PMC4464110/) | 2015 | Turn-taking is predictive, not reactive |
| 6 | [Saeki et al. - Multi-party AI turn-taking (Frontiers in AI)](https://www.frontiersin.org/journals/artificial-intelligence/articles/10.3389/frai.2025.1582287/full) | 2025 | SSJ rules work for AI agents in group conversation |
| 7 | [Bryant, Romero & du Boulay - Navigator role](https://www.sciencedirect.com/science/article/abs/pii/S1071581907000456) | 2008 | Field study of what navigators actually do |
| 8 | [Fowler - On Pair Programming](https://martinfowler.com/articles/on-pair-programming.html) | 2020 | 5-second rule, parking lot pattern, strategic vs tactical |
| 9 | [Intercom - Bot Design Principles](https://www.intercom.com/blog/principles-bot-design/) | 2021 | Behavioral triggers, non-intrusive placement |
| 10 | [Park et al. - Generative Agents (Stanford)](https://arxiv.org/abs/2304.03442) | 2023 | Memory retrieval with recency/relevance weighting |
