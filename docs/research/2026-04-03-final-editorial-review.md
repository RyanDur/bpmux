# Final Editorial Review

*April 3, 2026. Fourth and final editorial pass across the full research corpus (60+ documents). Covers fix verification from prior reviews, factual accuracy spot-checks, citation verification, internal contradictions, and preprint/peer-review status.*

---

## 1. Previously Flagged Issues: Fix Status

### Resolved (confirmed fixed in source files)

1. **Weiser & Brown date (awareness-design.md):** Now correctly reads "Weiser & Brown 1996" throughout. Fixed after being flagged three times.

2. **Grasse date (inspirations.md):** Now correctly reads "coined by Pierre-Paul Grasse in 1959." Fixed after being flagged three times.

3. **Bessen teller employment (inspirations.md):** Now reads "from ~500,000 to over ~550,000." Fixed after being flagged three times.

4. **MemGPT date conflation (memory-architecture.md):** The section header is now correctly "Packer et al., 2023; peer-reviewed at ICLR 2024." Fixed.

5. **ARAG link format (memory-architecture.md):** The URL now uses the standard `/abs/` path: `https://arxiv.org/abs/2506.21931`. Fixed.

6. **Storey arXiv ID (cognitive-intent-debt.md):** Verified. arXiv:2603.22106 resolves to Margaret-Anne Storey's "From Technical Debt to Cognitive and Intent Debt," submitted March 23, 2026, revised March 27, 2026. The ID is correct.

7. **Automation bias vs. overreliance (new-risks.md):** Now explicitly distinguishes the two: "Two related but distinct failure modes. Automation bias = favoring AI suggestions over contradictory evidence. Overreliance = failing to verify AI output at all." The distinction and different mitigations are articulated. Fixed.

### Partially fixed

8. **MemGPT in references section (memory-architecture.md):** The section header was corrected (item 4 above), but the Sources section at line 155 still reads `[MemGPT (2023)](https://arxiv.org/abs/2310.08560) -- peer-reviewed` without specifying ICLR 2024. The header and references are inconsistent with each other.

### Not verified as fixed (distillation issues from prior reviews)

9. **"Creation task" caveat (counter-arguments.md):** The April 2 editor notes flagged that `counter-arguments.md` uses the Vaccaro "creation task" finding without the "coding is mixed" qualifier present in `risk-mitigations.md`. The file still presents the creation-task classification in its opening paragraph without the qualifier. The caveat still does not travel with the claim.

10. **Sutton's scope (bitter-lesson.md):** The April 2 editor notes flagged that "70 years of AI research" overstates Sutton's coverage. A grep for "70 years" in the file returns no results, suggesting this was either fixed or was not present in the exact form flagged. No further action needed.

---

## 2. Citation Verification: Papers That Exist

All three papers specifically questioned by the review request have been verified as real:

**Koala study:** "Controlling AI Agent Participation in Group Conversations: A Human-Centered Approach" (Kim et al., IUI 2025). arXiv:2501.17258. Published in Proceedings of the 30th International Conference on Intelligent User Interfaces, March 24-27, 2025, Cagliari, Italy. The 72.2% (13/18) reactive preference figure, the proactive mode statistics, and the Koala II follow-up with user controls are all from this paper. **Confirmed real, peer-reviewed.**

**Inner Thoughts:** "Proactive Conversational Agents with Inner Thoughts" (Liu et al., CHI 2025). arXiv:2501.00383. Published at the CHI Conference on Human Factors in Computing Systems, April-May 2025, Yokohama, Japan. Available in ACM Digital Library (doi:10.1145/3706598.3713760). The motivation scoring framework and the 82% preference finding are from this paper. **Confirmed real, peer-reviewed.**

**BCG "AI brain fry" study:** "When Using AI Leads to Brain Fry" (Bedard, Kropp, Hsu, Karaman, Hawes, Kellerman). Published March 5, 2026 via BCG and Harvard Business Review. The 1,488-worker sample, the 14% incidence rate, the 12% mental fatigue increase for oversight tasks, and the productivity peak at 1-3 tools are all confirmed. **Confirmed real, published in HBR (industry study, not peer-reviewed academic research).** The developer-psychology document correctly identifies it as "a March 2026 BCG study" but does not note that it is an industry survey, not a peer-reviewed study. Given how prominently it is used, the evidence tier should be labeled.

---

## 3. Must Fix -- New Accuracy Issues

### 3a. ARAG paper is mischaracterized (memory-architecture.md)

**Problem:** The document cites ARAG (arXiv:2506.21931) as research on hybrid retrieval strategies for memory architecture, describing it as comparing "recency," "relevance," and "agentic RAG" strategies. The actual paper -- "ARAG: Agentic Retrieval Augmented Generation for Personalized Recommendation" (Walmart Global Tech, June 2025) -- is about **product recommendation systems**, not about memory or context management for AI agents. The RIR (Recency + Importance + Relevance) hybrid scoring described in the memory-architecture document may not come from this paper at all.

Additionally, the Sources section labels ARAG as "peer-reviewed," but it is an arXiv preprint with no published venue listed.

**Fix:** Verify whether the ARAG paper actually contains the hybrid scoring comparison described. If not, find the correct source or remove the citation. Either way, change "peer-reviewed" to "preprint" in the Sources section.

### 3b. "95% of professional developers now use AI coding tools weekly" is likely wrong (economics-business.md)

**Problem:** The economics-business document attributes this statistic to the "Anthropic 2026 Agentic Coding Trends Report." Summaries and coverage of that report describe engineers using AI in "roughly 60% of their work" and being able to "fully delegate" only 0-20% of tasks. The 95% figure does not appear in accessible summaries of the report and may be fabricated or conflated with a different source. The Stack Overflow 2025 survey says 84% of developers use or plan to use AI tools (a different and lower figure measuring a different thing).

**Fix:** Verify the 95% figure against the actual Anthropic report PDF. If not found, replace with the correct statistic and source, or remove the claim.

### 3c. Friedrich Kurz is a blog post, not a study (collaboration-problems.md)

**Problem:** The document says "Research comparing human-human pairs, human-AI pairs, and solo-plus-AI developers found that the highest performance came from human+AI pairs working together" and cites Friedrich Kurz (2025). The source is a personal blog post: `https://friedrichkurz.me/posts/2025-03-25-pair-programming-in-the-age-of-ai/`. This is being treated as research with the phrasing "Research comparing..." and "found that..." when it is one person's analysis on a blog.

The wave-2 counter-arguments document correctly identifies the tension between this claim and the Nature Human Behaviour meta-analysis finding, but does not note that Kurz is a blog post while Nature is a peer-reviewed meta-analysis of 370 results.

**Fix:** Change "Research comparing..." to "A practitioner analysis by Friedrich Kurz (2025) argues that..." or similar phrasing that accurately represents the source type. Label the source as a blog post.

---

## 4. Should Fix -- Accuracy and Evidence Quality

### 4a. Preprint vs. peer-reviewed status is inconsistently labeled across documents

Several citations are labeled "peer-reviewed" when they are arXiv preprints or industry reports. The networked-memory-security document does this well (using explicit evidence quality labels). Other documents do not. Specific cases:

| Document | Citation | Claimed status | Actual status |
|---|---|---|---|
| memory-architecture.md | ARAG (2506.21931) | "peer-reviewed" | arXiv preprint |
| memory-architecture.md | MemGPT (2310.08560) | "peer-reviewed" (in Sources) | Preprint; peer-reviewed at ICLR 2024 (header is correct, sources are not) |
| developer-psychology.md | BCG "AI brain fry" | No label | Industry survey published in HBR (not peer-reviewed academic research) |
| cognitive-intent-debt.md | Storey (2603.22106) | No explicit label | arXiv preprint, March 2026 |
| actual-developer-behavior.md | Multiple arXiv papers | Mix of labeled and unlabeled | Some are peer-reviewed (ICSE, CHI), some are preprints |

The wave-2 counter-arguments document correctly calls out that Storey's framework is "a 2026 preprint that has had roughly one month of community discussion." But the documents that rely on the framework do not carry this caveat.

**Fix:** Add evidence quality labels (peer-reviewed / preprint / industry report / blog post) to the Sources sections of memory-architecture.md, developer-psychology.md, and economics-business.md. The networked-memory-security format is the model.

### 4b. Collaborative Memory paper (arXiv:2505.18279) is also a preprint

The evening-editor-review document calls the Collaborative Memory paper a "citation" without noting its status. The paper was submitted May 23, 2025 by researchers at Accenture. It is an arXiv preprint, not peer-reviewed at a venue. The networked-memory-security document describes it as "required reading for v2 design" -- fine, but its status as an unreviewed preprint should be noted given how heavily it informs the security analysis.

### 4c. The "67% junior hiring collapse" is a composite statistic

The wave-2 counter-arguments document already flags this comprehensively: the 67% figure comes from ThinkPol and CodeConductor (not primary research institutions), and is conflated with the LeadDev survey (intentions, not actual hiring) and a separate study about AI-exposed roles generally. Despite this flagging, the evening-synthesis documents and the extracted-essentials document continue to cite "67% collapse in junior hiring" as a single fact. The caveat from the counter-arguments has not propagated.

**Fix:** Add a qualifier wherever the 67% appears outside the counter-arguments document: "67% decline in junior developer hiring since 2022 (per industry surveys; see counter-arguments for source limitations)."

---

## 5. Internal Contradictions

### 5a. The Kurz claim vs. the Nature meta-analysis

**Contradiction:** The collaboration-problems document says human+AI pairs outperform both human-human pairs and solo+AI developers (citing Kurz's blog post). The counter-arguments document says human-AI combinations "performed significantly worse than the best of humans or AI alone" on decision-making tasks (citing the Nature Human Behaviour meta-analysis of 370 results from 106 experiments).

The wave-2 counter-arguments document identifies this tension but says the corpus "resolves the tension by ignoring it -- citing Kurz when it supports the narrative and citing Nature when discussing risks, without acknowledging they contradict each other."

**Status:** Still unresolved. These two claims appear in different documents without cross-reference. A reader who encounters one will not see the other.

**Fix:** Add a note in collaboration-problems.md where Kurz is cited: "Note: the Nature Human Behaviour meta-analysis found human-AI teams underperform on decision-making tasks. Kurz's finding may reflect the creation-task advantage. See counter-arguments for the full tension."

### 5b. "Campfire" vs. "not a chat app"

**Mild tension:** The evening-synthesis-v2 says "bpmux is a campfire, not a chat app." The evening-synthesis says "bpmux is not a chat app. It is collective memory for human-AI teams." The adversarial-synthesis says "the campfire does not care how many people sit around it." But the v1 spec, the story candidates, and the TUI implementation describe a chat application with message input, display, and real-time exchange.

This is an identity tension, not a factual contradiction. But it means the corpus speaks about bpmux in two registers -- the theoretical register (shared reasoning environment, cognitive infrastructure) and the implementation register (chat app with a TUI). The wave-3 counter-arguments call this out: "This is a product that does not yet know what it is."

**No fix required.** This is a product strategy question, not a factual error. But the tension should be acknowledged rather than papered over.

### 5c. "~80-120 lines" vs. "~100-150 lines" for the WebSocket endpoint

**Minor inconsistency:** The external-peer-model document estimates the WebSocket endpoint at "~80-120 lines of new Rust code." The evening-synthesis says "roughly 150-200 lines of new Rust code." The evening-synthesis-v2 says "100-150 lines." The extracted-essentials says "~100-150 lines." These are rough estimates and the variation is within noise, but if a single canonical estimate is needed, the documents should converge.

---

## 6. Statistics That Appear With Different Numbers

### 6a. Koala reactive preference: 72% vs. 72.2%

The ai-turn-taking document correctly reports "72.2% (13/18)" from the paper. All other documents round to "72%." This is acceptable rounding but worth noting for precision: the sample was 18 participants, so the percentage is 13/18 = 72.2%. The rounded "72%" is used consistently elsewhere and is fine.

### 6b. METR perception gap: consistent

The METR numbers are cited consistently across all documents: 24% predicted speedup, 20% believed speedup after the fact, 19% actual slowdown. No contradictions found.

### 6c. Faros AI PR review time: consistent

The 91% longer PR review times figure from Faros AI (10,000 developers across 1,255 teams) is cited consistently. No contradictions found.

---

## 7. Research Cited as Established That Is Actually Recent/Unvalidated

| Citation | Status | How it is treated | Risk level |
|---|---|---|---|
| Storey Triple Debt Model (2603.22106) | arXiv preprint, March 2026 | Load-bearing theoretical foundation for the entire project | **High.** The wave-2 counter-arguments correctly flag this. The framework is one month old, unreplicated, and the entire bpmux value proposition rests on it. |
| Collaborative Memory (2505.18279) | arXiv preprint, May 2025 | "Required reading for v2 design" | Medium. Used for future planning, not current design. |
| ARAG (2506.21931) | arXiv preprint, June 2025 | Cited as evidence for hybrid retrieval strategies | **Medium-High.** The paper is mischaracterized (see 3a above). |
| BCG "AI brain fry" (March 2026) | Industry survey in HBR | Used as evidence for decision fatigue risk | Medium. Real study, but industry methodology, not peer-reviewed. |
| Codellaborator (CHI 2025) | Peer-reviewed at CHI 2025 | Cited for proactive AI findings | Low. Correctly attributed, peer-reviewed. |
| Koala (IUI 2025) | Peer-reviewed at IUI 2025 | Cited for reactive preference finding | Low. Correctly attributed, peer-reviewed. Small sample (N=18). |
| Inner Thoughts (CHI 2025) | Peer-reviewed at CHI 2025 | Cited for motivation scoring framework | Low. Correctly attributed, peer-reviewed. |
| METR study (arXiv:2507.09089) | arXiv preprint, July 2025 | Cited as the strongest empirical finding on AI productivity | Medium. Real study, methodologically sound, but preprint. 16 developers is a small sample. |
| Faros AI report (2025) | Industry report | 91% PR review time increase | Medium. Industry data, not peer-reviewed. Large sample (10,000 devs) lends credibility. |
| JetBrains ICSE 2026 (2601.10258) | Accepted at ICSE 2026 | Telemetry-based behavioral changes | Low. Peer-reviewed at a top venue. |
| Anthropic skill formation (2601.20245) | arXiv preprint, January 2026 | 17% lower comprehension, engagement patterns | Medium. From Anthropic's own research team, not independently peer-reviewed. |

The general pattern: the corpus's most load-bearing claims (Storey's debt model, the METR perception gap, the Faros review bottleneck) rest on preprints and industry reports. The peer-reviewed citations (Koala, Inner Thoughts, Codellaborator, JetBrains) support secondary claims about AI interaction design. This is not unusual for a fast-moving field, but the evidence hierarchy should be made explicit rather than left implicit.

---

## 8. The Remaining Unfixed Pattern

The main finding from the April 1 editor notes -- "caveats do not travel with the claim" -- persists into the April 3 corpus. Specific instances:

1. **The "creation task" caveat** (risk-mitigations says "coding is mixed"; counter-arguments does not repeat this). Flagged April 1. Still unfixed.

2. **The Kurz-vs-Nature contradiction** (collaboration-problems cites Kurz without the Nature caveat; counter-arguments-wave2 identifies the contradiction). Flagged April 2. Still unresolved.

3. **The 67% junior hiring caveat** (counter-arguments-wave2 correctly identifies the composite nature of the statistic; evening-synthesis, adversarial-synthesis, and extracted-essentials cite it without qualification). Flagged April 2. Caveat has not propagated.

4. **Storey's preprint status** (counter-arguments-wave2 correctly calls it a one-month-old unreplicated framework; all other documents that use the framework do not carry this caveat). Flagged April 2. Caveat has not propagated.

The wave-3 counter-arguments document states this pattern most bluntly: "The research accumulates evidence for the thesis at a ratio of roughly 45:2 compared to evidence against it." The caveats exist. They live in the counter-arguments documents. They do not travel to the documents that make the claims.

---

## 9. Summary of Required Actions

### Must fix (factual errors)

1. ARAG citation in memory-architecture.md is mischaracterized and mislabeled as peer-reviewed (Section 3a)
2. "95% of professional developers" statistic in economics-business.md needs verification or replacement (Section 3b)
3. Friedrich Kurz blog post is presented as research in collaboration-problems.md (Section 3c)

### Should fix (evidence quality and consistency)

4. Add evidence quality labels to Sources sections of memory-architecture.md, developer-psychology.md, economics-business.md (Section 4a)
5. Fix MemGPT label inconsistency between header and Sources section in memory-architecture.md (Section 1, item 8)
6. Add "source limitations" qualifier to the 67% junior hiring figure outside the counter-arguments document (Section 4c)

### Should resolve (internal contradictions)

7. The Kurz-vs-Nature contradiction needs cross-referencing (Section 5a)
8. The "creation task" caveat still does not travel to counter-arguments.md (Section 1, item 9)

### Acknowledge (structural issues the research itself identifies)

9. The transparency paradox is an unresolved design contradiction, not a calibration challenge (wave-3 counter-arguments)
10. Storey's framework is load-bearing but unvalidated (wave-2 counter-arguments)
11. The product identity ("campfire" vs. "chat app") remains split (Section 5b)
12. The WebSocket line-count estimates vary from 80 to 200 across documents (Section 5c)

---

## 10. Overall Assessment

The factual foundation of the research corpus is strong. The three papers specifically questioned (Koala, Inner Thoughts, BCG "AI brain fry") are all real and correctly described. The persistent issues from April 1 (Weiser date, Grasse date, Bessen figures) have finally been fixed. The METR, Faros AI, and Koala statistics are cited consistently across documents.

The primary remaining problems are:

1. **One mischaracterized citation** (ARAG) that needs correction or replacement.
2. **One likely fabricated statistic** (the 95% figure) that needs verification.
3. **One blog post presented as research** (Kurz) that needs honest labeling.
4. **Inconsistent evidence quality labeling** -- some documents label preprints and industry reports explicitly, most do not.
5. **Caveats that do not propagate** -- the counter-arguments documents are intellectually honest about limitations, but the primary documents that make the claims do not carry those caveats.

None of these are catastrophic. The corpus is honest where it matters most: the counter-arguments and adversarial-synthesis documents are genuine challenges, not strawmen. The extracted-essentials document correctly identifies what survives skepticism. The wave-3 counter-arguments are right that the research has become disproportionate to the building -- but as a research corpus, it is thorough, well-sourced, and more self-critical than most.
