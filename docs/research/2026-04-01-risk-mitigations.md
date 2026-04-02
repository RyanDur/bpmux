# Risk Mitigations: Research-Supported Strategies

*For each identified risk, what does research say about turning it into a strength or mitigating it? Only claims with cited sources.*

---

## Risk 1: Commitment Deficits — Humans Disengage When Working with AI

**The risk:** A 2025 study found reduced commitment in human-AI teams compared to human-only teams.

**What research says about mitigation:**

**Accountability through role clarity.** Lou et al. (2025, "Unraveling Human-AI Teaming," arXiv:2504.05755) found that clear role specification — well-defined roles based on expertise — provides structure that supports effective team coordination. Ambiguous responsibility distribution undermines collaboration. The recommendation: explicit role assignments with adaptive flexibility.

**bpmux already does this.** The Ground Control / Major Tom agreement is explicit role specification: human decides, AI implements, human approves. This isn't just a process document — it's a research-supported mitigation for the commitment deficit. The key addition: **make the role assignment visible in the tool**. The TUI/Web UI could display the current working agreement alongside the chat, reinforcing who owns what.

**Feedback systems.** The same review identifies four types of feedback (descriptive, normative, corrective, evaluative) as critical for maintaining engagement. bpmux's visibility markers and inactivity detection are descriptive feedback. The working agreement's review cycle (Opus reviews → Ground Control decides) is evaluative feedback.

**From risk to strength:** If bpmux's visible role assignments and feedback systems *reduce* commitment deficits compared to working with AI in a tool that has no explicit role clarity (raw Claude Code without a working agreement), then bpmux doesn't just mitigate the risk — it becomes the *solution* to the commitment deficit. The hypothesis: structured, visible collaboration roles keep humans engaged where unstructured AI interaction lets them drift.

**Source:** [Lou et al. (2025) — Unraveling Human-AI Teaming](https://arxiv.org/html/2504.05755v2)

---

## Risk 2: Awareness Overload — Too Much Transparency Overwhelms

**The risk:** CSCW research warns that awareness information has a cost. Full transparency may overwhelm when AI-to-AI chatter is high-volume.

**What research says about mitigation:**

**Peripheral awareness displays.** Mark Weiser's "calm technology" concept and subsequent CSCW research on ambient displays show that information can be presented at different attention levels: focal (demands attention), peripheral (available but not intrusive), and ambient (background awareness). The key insight: not all information needs the same salience.

**bpmux design response:** AI-to-AI messages don't need to be focal — they can be **peripheral**. The TUI/Web UI could render AI-to-AI messages in a visually distinct, lower-salience style (dimmed text, collapsed by default, expandable on demand). The human sees that conversation is happening (ambient awareness) without being forced to read every message (focal attention). This is supported by research showing that peripheral displays "help users stay aware of information without being overly intrusive or distracting."

**Information filtering strategies.** A 2023 comprehensive review in *Frontiers in Psychology* identifies four overload coping strategies: ignoring, filtering, customizing, and saving. The relevant ones for bpmux: **filtering** (the token-aware read window already does this for the AI) and **customizing** (letting the human configure what they see). The research recommends "establishing common rules (etiquette in dealing with ICT)" as a team-level strategy — which maps to the working agreement.

**From risk to strength:** If bpmux implements peripheral rendering for AI-to-AI messages, it becomes a tool that provides *more* awareness with *less* cognitive cost than reading raw Claude Code output. The research says ambient awareness is valuable when it's well-designed. Full transparency + intelligent rendering = awareness without overload.

**Sources:**
- [Dealing with information overload: a comprehensive review (Frontiers in Psychology, 2023)](https://pmc.ncbi.nlm.nih.gov/articles/PMC10322198/)
- [Exploring the design of peripheral displays of awareness information](https://www.researchgate.net/publication/221513841)

---

## Risk 3: Human-AI Teams Underperform on Decisions

**The risk:** Nature Human Behaviour 2024 meta-analysis found human-AI combinations perform worse than the best of either alone, especially on decision-making tasks.

**What research says about mitigation:**

**AI delegates to humans, not the other way around.** Research published in *Information Systems Research* (Kühl et al., 2022) found a striking asymmetry: human-AI team performance improves when AI delegates work to humans, but NOT when humans delegate to AI. Even when AI delegated to low-performing humans, combined performance improved. By contrast, "humans did not delegate well and did not benefit from delegation to the AI" because humans cannot accurately assess their own capability relative to task difficulty.

**bpmux's working agreement already implements this pattern — partially.** Ground Control (human) makes all decisions, but the AI proposes and implements. The research suggests the optimal direction is actually: **AI identifies what needs human judgment and escalates**. The `read_messages` / `send_message` pattern supports this — the AI can surface decisions for human review through the chat, effectively delegating judgment calls to the human.

**Creation tasks show positive effects.** The Nature study found positive effects for creation tasks (g = 0.19). Coding involves substantial creation (writing new code, generating solutions). The study's authors hypothesize creation tasks succeed because "even when they require creativity, they also involve substantial amounts of routine generation" — which describes AI-assisted coding well.

**Predetermined division of labor is promising but understudied.** Only 3 of 106 studies in the meta-analysis explored predetermined division of labor. Those 3 showed promise. bpmux's working agreement is an explicit, predetermined division of labor. This doesn't prove it works, but it places bpmux in the small set of approaches the research identifies as promising.

**From risk to strength:** The research says human-AI teams fail when roles are ambiguous and humans make final calls on tasks where AI is more accurate. bpmux's design inverts this for the dangerous cases: on coding tasks (creation), the AI generates and the human reviews. On architecture decisions (judgment), the human leads and the AI advises through the chat. The visible, persistent record of reasoning in the chat means the human can review AI decisions with full context — addressing the "AI confidence display" recommendation from the meta-analysis.

**Sources:**
- [Kühl et al. (2022) — Cognitive Challenges in Human-AI Collaboration: Productive Delegation (Information Systems Research)](https://pubsonline.informs.org/doi/10.1287/isre.2021.1079)
- [Vaccaro et al. (2024) — When combinations of humans and AI are useful (Nature Human Behaviour)](https://www.nature.com/articles/s41562-024-02024-1)

---

## Risk 4: Communication Overhead Scales Non-Linearly

**The risk:** Brooks' Law — coordination costs grow faster than team size.

**What research says about mitigation:**

**Stigmergy reduces communication overhead.** This is directly from Crowston's research (2016). In FLOSS teams, 14/20 multi-developer tasks required zero direct communication. Stigmergic coordination through the shared artifact *replaces* explicit communication, not adds to it. The coordination overhead that Brooks describes comes from direct communication channels (n(n-1)/2). Stigmergic coordination doesn't create pairwise channels — it uses one shared environment that all participants read from.

**bpmux is stigmergic by design.** The message log is a shared environment, not a set of pairwise channels. Adding a third or fourth participant doesn't create new communication pairs — everyone reads from and writes to the same log. The coordination cost scales linearly (each participant reads the log) not quadratically (each participant talks to every other).

**From risk to strength:** Brooks' Law applies to systems with pairwise communication channels. Stigmergic systems scale differently — through one shared environment rather than n(n-1)/2 channels. bpmux's architecture is inherently resistant to the coordination overhead that Brooks describes. This isn't a claim — it's a structural property of the design, supported by Crowston's finding that stigmergic coordination works without direct communication.

**Source:** [Bolici, Howison & Crowston (2016) — Stigmergic coordination in FLOSS development teams](https://www.sciencedirect.com/science/article/pii/S1389041715000339)

---

## Summary

| Risk | Mitigation Strategy | Research Support | bpmux Implementation | Status |
|---|---|---|---|---|
| Commitment deficits | Explicit role specification + visible feedback | Lou et al. 2025 (arXiv) | Working agreement + visibility markers | Already designed; add role display to TUI/Web UI |
| Awareness overload | Peripheral rendering for AI-to-AI messages | Weiser (calm tech), Frontiers in Psych 2023 | Not yet designed | Add peripheral rendering as v1.x feature |
| Decision underperformance | AI-to-human delegation + creation tasks + predetermined labor division | Kühl 2022 (ISR), Vaccaro 2024 (Nature) | Working agreement + chat-based escalation | Already designed; strengthen AI escalation pattern |
| Communication overhead | Stigmergic coordination (shared environment, not pairwise channels) | Crowston 2016 (Cognitive Systems Research) | Message log architecture | Already designed — structural property |

**The pattern:** Three of four risks have mitigations that bpmux already implements (working agreement, visibility markers, stigmergic architecture). The fourth (awareness overload) needs a design addition: peripheral rendering for AI-to-AI messages in the TUI/Web UI. This is a small, well-defined feature that can be added as a v1.x iteration informed by dogfooding.
