# Risk Mitigations: Research-Supported Strategies

*For each identified risk, what does research say about turning it into a strength or mitigating it? Only claims with cited sources.*

---

## Risk 1: Commitment Deficits — Humans Disengage When Working with AI

**The risk:** A 2025 study found reduced commitment in human-AI teams compared to human-only teams.

**What research says about mitigation:**

**Accountability through role clarity.** Lou et al. (2025, "Unraveling Human-AI Teaming," arXiv:2504.05755) found that clear role specification — well-defined roles based on expertise — provides structure that supports effective team coordination. Ambiguous responsibility distribution undermines collaboration. The recommendation: explicit role assignments with adaptive flexibility.

**What bpmux provides (tool features):** Visibility markers (descriptive feedback on who has seen what), inactivity detection (signals when a participant goes silent), sender identity (makes clear who said what). These are features that *support* awareness but do not enforce role discipline.

**What the working agreement provides (process):** Role specification (Ground Control decides, AI implements), human sign-off gates, review chains. The working agreement is how the team using bpmux chooses to work — it is not a feature of bpmux itself.

**The gap:** bpmux provides awareness. The working agreement provides accountability. Research says you need both. bpmux cannot take credit for the process discipline of its users. A team using bpmux without a working agreement would still have the commitment deficit risk — bpmux's transparency helps but doesn't solve it alone.

**What bpmux could add as a feature:** Display the team's working agreement in the TUI/Web UI sidebar — making process visible alongside conversation. bpmux could also surface role-related prompts (e.g., "Ground Control hasn't approved this yet") if the team configures their agreement into the tool. This would move role clarity from process-only to process-supported-by-tool. This is a hypothesis worth exploring in dogfooding, not a proven mitigation.

**Source:** [Lou et al. (2025) — Unraveling Human-AI Teaming](https://arxiv.org/html/2504.05755v2)

---

## Risk 2: Awareness Overload — Too Much Transparency Overwhelms

**The risk:** CSCW research warns that awareness information has a cost. Full transparency may overwhelm when AI-to-AI chatter is high-volume.

**What research says about mitigation:**

**Peripheral awareness displays.** Mark Weiser's "calm technology" concept and subsequent CSCW research on ambient displays show that information can be presented at different attention levels: focal (demands attention), peripheral (available but not intrusive), and ambient (background awareness). The key insight: not all information needs the same salience.

**bpmux design response:** AI-to-AI messages don't need to be focal — they can be **peripheral**. The TUI/Web UI could render AI-to-AI messages in a visually distinct, lower-salience style (dimmed text, collapsed by default, expandable on demand). The human sees that conversation is happening (ambient awareness) without being forced to read every message (focal attention). This is supported by research showing that peripheral displays "help users stay aware of information without being overly intrusive or distracting."

**Information filtering strategies.** Information overload literature identifies coping strategies including ignoring, filtering, customizing, and saving (taxonomy cited in a 2023 *Frontiers in Psychology* review, though the taxonomy may originate from earlier work). The relevant ones for bpmux: **filtering** (the token-aware read window already does this for the AI) and **customizing** (letting the human configure what they see). The research recommends "establishing common rules (etiquette in dealing with ICT)" as a team-level strategy — which maps to the working agreement.

**From risk to strength:** If bpmux implements peripheral rendering for AI-to-AI messages, it becomes a tool that provides *more* awareness with *less* cognitive cost than reading raw Claude Code output. The research says ambient awareness is valuable when it's well-designed. Full transparency + intelligent rendering = awareness without overload.

**Sources:**
- [Dealing with information overload: a comprehensive review (Frontiers in Psychology, 2023)](https://pmc.ncbi.nlm.nih.gov/articles/PMC10322198/)
- [Exploring the design of peripheral displays of awareness information](https://www.researchgate.net/publication/221513841)

---

## Risk 3: Human-AI Teams Underperform on Decisions

**The risk:** Nature Human Behaviour 2024 meta-analysis found human-AI combinations perform worse than the best of either alone, especially on decision-making tasks.

**What research says about mitigation:**

**AI delegates to humans, not the other way around.** Research published in *Information Systems Research* (Kühl et al., 2022) found a striking asymmetry: human-AI team performance improves when AI delegates work to humans, but NOT when humans delegate to AI. Even when AI delegated to low-performing humans, combined performance improved. By contrast, "humans did not delegate well and did not benefit from delegation to the AI" because humans cannot accurately assess their own capability relative to task difficulty.

**What bpmux provides (tool features):** A persistent, visible chat where AI reasoning is transparent. When an AI sends a message explaining its reasoning or asking for a decision, that's visible to the human in the TUI/Web UI. The chat is a natural escalation channel — the AI can surface decisions for human review. bpmux makes the AI's reasoning available; it doesn't control how the team divides labor.

**What the working agreement provides (process):** The division of labor (human decides, AI implements). Predetermined division of labor is identified by the Nature study as promising (only 3 of 106 studies explored it). The working agreement is an instance of this approach — but it's a process choice, not a bpmux feature.

**Creation tasks show positive effects, but coding is mixed.** The Nature study found positive effects for creation tasks (g = 0.19) but losses for decision-making tasks. Coding involves both: code generation is creation, but architecture choices, design tradeoffs, and bug triage are decision-making — exactly the category where human-AI teams underperform. The study doesn't explicitly classify coding in their taxonomy. We cannot lean on "coding is a creation task" as a blanket justification. The honest framing: bpmux's working agreement attempts to route creation work to the AI and decision work to the human, but coding is inherently mixed.

**What bpmux could add as a feature:** The Kühl (2022) finding — that AI-to-human delegation outperforms human-to-AI delegation — suggests bpmux could support an explicit escalation pattern. An MCP tool like `request_decision(question, options)` would let the AI formally surface a judgment call to the human through the chat, with the question pinned or highlighted in the TUI/Web UI until the human responds. This moves decision escalation from ad-hoc chat messages to a structured feature. This is a hypothesis for dogfooding.

**From risk to possible strength:** bpmux's persistent chat makes AI reasoning visible — a prerequisite for effective human review of AI decisions. But visibility alone doesn't solve the decision underperformance problem. The working agreement (process) plus visible reasoning (tool) together address it. Neither alone is sufficient.

**Sources:**
- [Fugener, Grahl, Gupta & Ketter (2022) — Cognitive Challenges in Human-AI Collaboration: Productive Delegation (Information Systems Research)](https://pubsonline.informs.org/doi/10.1287/isre.2021.1079)
- [Vaccaro et al. (2024) — When combinations of humans and AI are useful (Nature Human Behaviour)](https://www.nature.com/articles/s41562-024-02024-1)

---

## Risk 4: Communication Overhead Scales Non-Linearly

**The risk:** Brooks' Law — coordination costs grow faster than team size.

**What research says about mitigation:**

**Stigmergy reduces communication overhead.** This is directly from Crowston's research (2016). In FLOSS teams, 14/20 multi-developer tasks required zero direct communication. Stigmergic coordination through the shared artifact *replaces* explicit communication, not adds to it. The coordination overhead that Brooks describes comes from direct communication channels (n(n-1)/2). Stigmergic coordination doesn't create pairwise channels — it uses one shared environment that all participants read from.

**bpmux is stigmergic by design.** The message log is a shared environment, not a set of pairwise channels. Adding a third or fourth participant doesn't create new communication pairs — everyone reads from and writes to the same log. The coordination cost scales linearly (each participant reads the log) not quadratically (each participant talks to every other).

**From risk to different scaling curve:** Brooks' Law describes quadratic overhead from pairwise channels. Stigmergy replaces this with linear reading overhead (each participant reads the shared log) plus a noise-filtering problem (more participants = more traces to parse). This is better, but not free. Token awareness and bounded windows are the cost-management mechanism for the reading overhead. The noise-filtering problem is addressed by message tiers (v1.x). Stigmergy doesn't eliminate coordination costs — it shifts them from communication overhead to information processing overhead, which is more tractable.

**Source:** [Bolici, Howison & Crowston (2016) — Stigmergic coordination in FLOSS development teams](https://www.sciencedirect.com/science/article/pii/S1389041715000339)

---

## Summary

| Risk | What bpmux provides (tool) | What the working agreement provides (process) | Research Support | Status |
|---|---|---|---|---|
| Commitment deficits | Visibility markers, inactivity detection, sender identity | Role specification, human sign-off gates | Lou et al. 2025 | Tool supports awareness; process provides accountability. Neither alone is sufficient. |
| Awareness overload | Bounded read windows, token awareness | — | Weiser (calm tech), Frontiers in Psych 2023 | Add peripheral rendering for AI-to-AI messages (v1.x) |
| Decision underperformance | Persistent visible chat (AI reasoning transparent) | Division of labor, human decides | Fugener et al. 2022 (ISR), Vaccaro 2024 (Nature) | Explore `request_decision` MCP tool for structured escalation |
| Communication overhead | Stigmergic architecture (shared log, not pairwise channels) | — | Crowston 2016 | Structural property of the design |

**The honest picture:** bpmux mitigates two risks structurally (communication overhead via stigmergy, awareness overload via bounded windows + future peripheral rendering). For the other two (commitment deficits, decision underperformance), bpmux provides *supporting* features (visibility, transparency) but the actual mitigation comes from the team's process discipline. The tool makes good process easier to follow — it doesn't replace it.
