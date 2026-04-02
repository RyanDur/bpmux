# Known Risks: Research-Informed Constraints on Stigmergic Coordination

*Compiled April 1, 2026. These aren't arguments against bpmux's approach — they're properties of stigmergic systems that need to be managed.*

---

## 1. Human-AI Teams Often Perform Worse Than AI Alone

**The research:** A 2024 Nature Human Behaviour meta-analysis (370 results from 106 experiments) found that human-AI combinations performed **significantly worse than the best of humans or AI alone**. The losses were concentrated in **decision-making tasks** — where humans second-guess correct AI suggestions or fail to override incorrect ones. The only gains were in **creation tasks** (open-ended content generation).

**The challenge to bpmux:** bpmux adds MORE humans and MORE AIs to the loop. If a single human-AI pair already underperforms AI alone on decisions, does adding a second pair make it worse? More participants = more opportunities for someone to override a correct judgment or defer to an incorrect one.

**Our response:** The Nature study tested scenarios where humans and AI share a single decision point (e.g., "is this a deepfake?"). bpmux's model is different — it's **pair programming**, where humans and AIs have distinct roles with a clear division of labor (Ground Control decides, AI implements). The study found that "predetermined division of labor" showed promise but was understudied (only 3 studies). bpmux's working agreement IS a predetermined division of labor. The study found creation tasks showed positive effects — but coding is mixed: code generation is creation, while architecture choices, design tradeoffs, and bug triage are decision-making, exactly the category where human-AI teams underperform. We cannot lean on "coding is a creation task" as a blanket justification.

**Risk level:** Medium. We should watch for decision-making bottlenecks during dogfooding — moments where having two humans and two AIs makes decisions slower or worse than one human + one AI.

---

## 2. Pair Programming Is Not Uniformly Beneficial

**The research:** The Hannay meta-analysis found that pair programming is "not uniformly beneficial or effective." It increases total person-hours by ~15%. Quality improvements are task-complexity dependent. There are signs of publication bias in pair programming studies. Some developers report that pairing prevents deep thought — "you just don't go as deep, don't have certain insights."

**The challenge to bpmux:** If pair programming itself has mixed evidence, building infrastructure specifically for AI-assisted pair programming is betting on a contested practice. The developers who find pairing least valuable (those who need deep solo focus) are precisely the ones who won't use bpmux.

**Our response:** bpmux isn't exclusively a pairing tool. Solo mode (Phase 1 dogfooding) works for a single developer. The persistent message log has value even without a partner. And the "When to Use bpmux" section explicitly acknowledges that pairing isn't for every task. The risk is building features that only matter in pair mode and neglecting solo mode — we should ensure solo mode is a first-class experience, not a degraded version of pair mode.

**Risk level:** Low-Medium. Mitigated by solo mode being a first-class use case.

---

## 3. Communication Overhead Scales Non-Linearly

**The research:** Brooks' Law (The Mythical Man-Month, 1975): adding people to a late project makes it later. Communication pairs grow as n(n-1)/2. The general pattern — coordination overhead grows faster than team size — is well-established in software engineering literature, though specific percentages (often cited as 30-40% of capacity on coordination) come from industry surveys (McKinsey, Asana) rather than controlled studies.

**The challenge to bpmux:** bpmux ADDS communication channels. With 2 humans and 2 AIs, that's 6 possible communication pairs. The AIs generate messages continuously — each `send_message` call adds to everyone's context. If the AI-to-AI chatter is high-volume, the humans may drown in noise rather than gain clarity. The channel becomes a firehose.

**Our response:** Token awareness directly addresses this. The bounded read window (default 50 messages) prevents firehose. The inactivity detection distinguishes signal from silence. The "When to Use bpmux" section guides users away from low-value scenarios. But this is a real risk — if the AIs are chatty, the noise-to-signal ratio could make the channel less useful than no channel at all. Dogfooding will reveal whether the default window size is right.

**Risk level:** Medium. The token awareness design mitigates this, but the right defaults can only be discovered through usage.

---

## 4. Reduced Commitment in Human-AI Teams

**The research:** A 2025 study found evidence of "commitment deficits" in human-AI teams — humans invest less effort when working with AI teammates compared to human teammates. The presence of AI as a collaborator can reduce human motivation and engagement.

**The challenge to bpmux:** If developers rely on the AI to carry context and make decisions, they may disengage. "The AI is watching the chat, I don't need to pay attention." This is especially risky in the solo orchestration mode — if Claude is coordinating with local models, the human might check out entirely, losing the "Ground Control" oversight that the working agreement depends on.

**Our response:** bpmux provides transparency (the human can see everything) and visibility markers (the human knows what the AI has seen). These keep the human *informed* but don't keep them *engaged* — that requires process discipline from the team, which is outside bpmux's scope. bpmux provides the information. Whether the human pays attention is up to them.

**Risk level:** Medium-High. This is the most concerning finding. bpmux mitigates the information problem (the human can see what's happening) but not the behavioral problem (the human choosing to look). The behavioral problem is a team process concern.

---

## 5. Transparency Can Be Overwhelming

**The research:** Dourish & Bellotti's foundational 1992 CSCW paper established that awareness of collaborators' activities is critical for coordination. Building on their awareness framework, later CSCW researchers (Hudson & Smith 1996, and the broader information overload literature — Frontiers in Psychology 2023) identified that too much awareness information leads to "awareness overload" — the cost of processing awareness information exceeds its value.

**The challenge to bpmux:** The transparency principle ("everything is visible to everyone") means the human sees ALL AI-to-AI communication. If two AIs are actively reasoning about the codebase, the chat stream could be overwhelming. The human might start ignoring it, defeating the purpose.

**Our response:** The bounded read window helps — the AI sees 50 messages, not everything. But the HUMAN sees everything in the TUI/Web UI. There's no corresponding "awareness budget" for the human view. In stigmergic terms, this is pheromone oversaturation — too many traces in the environment make it harder, not easier, to coordinate. Ant colonies solve this through pheromone evaporation (old signals fade). bpmux may need an equivalent — older messages fading visually, AI-to-AI chatter collapsible, or a focus mode that highlights only recent activity. Dogfooding will reveal the right balance.

**Risk level:** Medium. Not a tension between transparency and usability, but a signal-to-noise problem inherent in any stigmergic system. The traces need to be visible but not overwhelming.

---

## Summary: What This Means for bpmux

| Risk | Severity | Mitigation | Status |
|---|---|---|---|
| Human-AI teams underperform on decisions | Medium | bpmux provides visible reasoning for human review; division of labor is a team process decision | Partially mitigated (tool provides info, process provides discipline) |
| Pair programming has mixed evidence | Low-Medium | Solo mode is first-class; not for every task | Mitigated |
| Communication overhead | Medium | Token awareness, bounded windows | Mitigated by design, defaults TBD |
| Commitment deficits | Medium-High | Working agreement, human sign-off | Process control only — fragile |
| Awareness overload | Medium | Bounded AI window; human view TBD | Not yet mitigated for human view |

**The honest assessment:** None of this research says bpmux is the *wrong* approach. The risks are inherent to stigmergic systems, not specific to bpmux's design:

1. **Commitment deficits** — agents (human or AI) disengage when the environment seems to be self-coordinating. In ant colonies, this doesn't happen because ants are hardwired to respond to pheromones. Humans aren't. bpmux provides information (transparency, visibility markers) but the behavioral defense against disengagement is a team process concern, not a tool feature.

2. **Pheromone oversaturation** — too many traces in the environment make coordination harder, not easier. Token awareness and bounded windows are the evaporation mechanism. Dogfooding will calibrate the right decay rate.

3. **Human-AI complementarity is hard to achieve** — the research says it works best with predetermined division of labor. Coding is mixed: generation is creation (positive effects), but architecture and design decisions are decision-making (negative effects). How teams divide labor is a process choice. bpmux makes the AI's reasoning visible so the human can exercise judgment — but it doesn't enforce the division.

These should be tracked as explicit risks and watched during dogfooding. They're properties of the system type, not flaws in the design.

## Sources

- [When combinations of humans and AI are useful: A systematic review and meta-analysis](https://www.nature.com/articles/s41562-024-02024-1) (Nature Human Behaviour, 2024)
- [AI teammates and human performance: Evidence for commitment deficits](https://www.sciencedirect.com/science/article/pii/S245195882500243X) (2025)
- [The effectiveness of pair programming: A meta-analysis](https://www.sciencedirect.com/science/article/abs/pii/S0950584909000123) (2009)
- [Communication overhead: The hidden cost of team cognition](https://www.researchgate.net/publication/344477388_Communication_overhead_The_hidden_cost_of_team_cognition)
- [Research Shows Human-AI Shortcomings — MIT](https://ide.mit.edu/insights/research-shows-human-ai-shortcomings-and-how-to-fix-them/)
- [Awareness and Coordination in Shared Workspaces — Dourish & Bellotti](https://www.semanticscholar.org/paper/Awareness-and-coordination-in-shared-workspaces-Dourish-Bellotti/fc0ccc4e5c0f1f1f1eef1bc9a5a840d1a8e2fd95)
