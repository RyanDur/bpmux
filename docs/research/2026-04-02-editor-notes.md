# Editor Notes: April 2 Research Verification

*Reviewed 2026-04-02. Covers all 12 April 2 research documents plus the 3 unfixed items from the April 1 editor notes.*

**Overall: Strong.** The epistemic discipline from the April 1 docs carries through. The new documents maintain clear separation between research findings and bpmux hypotheses. No fabricated citations found. The networked memory security document is the most rigorous piece in the entire corpus. Six accuracy issues identified, three distillation improvements recommended.

---

## Must Fix — Accuracy

### 1. Weiser & Brown — Wrong Date (Carried from April 1 notes, still unfixed)

**File:** `awareness-design.md`
**Problem:** Section header says "Weiser & Brown 1995" — the paper ("The Coming Age of Calm Technology") was published **October 5, 1996**. Other files correctly use 1996.
**Fix:** Change "Weiser & Brown 1995" to "Weiser & Brown 1996" in the section header.

### 2. Pierre-Paul Grassé — Imprecise Date (Carried from April 1 notes, still unfixed)

**File:** `inspirations.md`
**Problem:** Says stigmergy was "coined by Pierre-Paul Grassé in the 1950s" — the correct year is **1959**. The April 1 editor notes' own confirmed claims list says "Grasse (1959)."
**Fix:** Change "in the 1950s" to "in 1959."

### 3. Bessen — Teller Employment Upper Bound (Carried from April 1 notes, still unfixed)

**File:** `inspirations.md`
**Problem:** Claims teller employment grew "from ~500,000 to ~600,000." Verification against Bessen's original data suggests the upper bound is closer to **~550,000**, not ~600,000.
**Fix:** Change "~600,000" to "~550,000" or soften to "from ~500,000 to over ~550,000."

### 4. MemGPT — Conflated Preprint Date with Peer Review Status

**File:** `memory-architecture.md`
**Problem:** Listed as "Packer et al., 2023 — peer-reviewed." The arXiv preprint was October 2023 (2310.08560). Peer review acceptance was at **ICLR 2024**. Calling it "2023 — peer-reviewed" conflates the preprint date with the peer review status.
**Fix:** Change to "Packer et al., 2023; peer-reviewed at ICLR 2024" or "MemGPT (Packer et al., ICLR 2024)."

### 5. ARAG Citation — Verify Link Format

**File:** `memory-architecture.md`
**Problem:** Cites "ARAG, 2025" with URL `https://arxiv.org/html/2506.21931`. The `/html/` path format is non-standard for arXiv — standard is `https://arxiv.org/abs/2506.21931`. The current URL may resolve but could break.
**Fix:** Verify the link resolves. If the standard `/abs/` URL works, use that instead: `https://arxiv.org/abs/2506.21931`.

### 6. Storey arXiv ID — Verify Exists

**File:** `cognitive-intent-debt.md`
**Problem:** Cites `arXiv:2603.22106`. The "2603" prefix means March 2026, which is plausible. However, this specific ID should be verified against the actual arXiv page to confirm it resolves to the Storey paper.
**Fix:** Open `https://arxiv.org/abs/2603.22106` and confirm it's the Storey paper. If not, find and correct the ID.

---

## Should Fix — Distillation

### 7. "Creation task" caveat still doesn't travel (Carried from April 1 notes)

**File:** `counter-arguments.md`
**Problem:** Uses the Vaccaro "creation task" finding without the "coding is mixed" qualifier that `risk-mitigations.md` correctly includes. A reader encountering the claim first in counter-arguments gets an unqualified version.
**Fix:** Add a parenthetical or footnote where the creation-task classification appears: "(Note: coding is inherently mixed — creation + decision-making. The creation-task finding doesn't apply cleanly.)" or similar, matching the language already in `risk-mitigations.md`.

### 8. Bitter Lesson — Sutton's Scope Slightly Overstated

**File:** `bitter-lesson.md`
**Problem:** Says Sutton observed this "across 70 years of AI research." Sutton's essay actually focuses on a handful of specific domains (chess, Go, speech recognition, computer vision). The lesson is *inferred* as general, but "70 years" implies comprehensive coverage across all AI research.
**Fix:** Change to "across multiple decades of AI research in domains like chess, Go, speech recognition, and computer vision" or "across several landmark AI domains spanning 70 years."

### 9. Automation Bias vs Overreliance — Distinct Risks Conflated

**File:** `new-risks.md`
**Problem:** Risk #3 treats automation bias and overreliance as essentially the same, citing OWASP LLM09 (overreliance). These are related but distinct:
- **Automation bias** = favoring automated suggestions over contradictory evidence (you see the problem but defer to the AI)
- **Overreliance** = failing to verify AI output at all (you don't look for problems)

The distinction matters because mitigations differ: bias requires training humans to notice contradictions; overreliance requires verification workflows.
**Fix:** Split into two sentences acknowledging the distinction, or add a note: "Automation bias (deferring to AI despite contradictory evidence) and overreliance (failing to verify) are related but distinct failure modes with different mitigations."

---

## Consider — Author's Discretion

### 10. Jazz Analogy — Missing the Strongest Implication

**File:** `new-inspirations.md`
**Problem:** The jazz analogy correctly notes that improvisation requires skilled participants. But it misses a deeper point: jazz musicians share a harmonic and rhythmic vocabulary (chord changes, time signatures, genre conventions) that *constrains* their improvisation. Without shared constraints, you get noise, not music. bpmux participants don't share an equivalent constraint structure — unless the working agreement provides one.

This actually argues *for* the working agreement being essential infrastructure, not optional process. The analogy supports the spec's design more strongly than the document realizes.
**Suggestion:** Add a sentence: "Jazz also shows that shared constraints (harmonic vocabulary, time signature) are what make improvisation productive rather than chaotic. The working agreement serves this role in bpmux — it's the shared vocabulary that makes unplanned coordination possible."

### 11. Surveillance Risk — Not Fully Addressed by Log Access Controls

**File:** `responsible-ai.md`
**Problem:** The surveillance analysis is strong and the design principles (participant-owned logs, no management dashboard) are correct. But the hardest version of the problem isn't addressed: even if the *tool* prevents management access to logs, the *output* (code, commits, PRs, velocity changes) that results from bpmux sessions is visible. A manager can infer AI usage patterns from commit frequency, code style shifts, and PR throughput. The surveillance risk extends beyond log access.
**Suggestion:** Add a paragraph acknowledging this: "Log access controls address direct surveillance. They do not address indirect inference — managers can observe behavioral changes (commit patterns, code style, velocity) that reveal AI usage. This is a limitation of any tool-level protection; the broader organizational culture around AI use is outside bpmux's control but relevant to its adoption."

---

## Confirmed — No Issues Found

The following documents had no accuracy or significant distillation issues:

- **`a2a-protocol.md`** — Clean comparison. Correctly identifies A2A as not relevant for v1, worth watching for v2+. The "MCP for tools, A2A for tasks, bpmux for understanding" framing is crisp.
- **`distributed-cognition.md`** — Well-sourced. Hutchins, Norman, Hollnagel & Woods chain is solid. The "boundary object" application (Star & Griesemer) is the strongest theoretical contribution.
- **`networked-memory-security.md`** — The most rigorous document in the corpus. Evidence quality labels (peer-reviewed / industry / hypothesis) are exactly right. The regulatory analysis is appropriately alarming. The "honest assessment" that cross-org memory sharing may not be feasible is credible.
- **`measurement.md`** — Correctly refuses to claim statistical significance from n=1-2. The "state the observation, not the conclusion" principle and denominator reporting are exactly right. Miller citation is appropriate.
- **`story-candidates.md`** — Well-sliced, incrementally buildable, honestly labeled as candidates. Sequencing diagram is clear.
- **`new-risks.md`** — (Apart from #9 above) Genuinely new risks not covered in April 1. The JSONL log growth risk citing Claude Code's own bug is a strong find. Evidence quality is appropriately labeled.
- **`new-inspirations.md`** — (Apart from #10 above) Well-ranked by relevance. The synthesis ("infrastructure for a joint cognitive system") is the deepest framing in the corpus.

---

## The Strongest Contribution

The **Bitter Lesson synthesis** (lines 62-70 of `bitter-lesson.md`) threading Sutton and Storey together is the most original intellectual contribution across both days of research. The framing — "general-purpose shared environment (Sutton-aligned) that captures reasoning as a side effect (Storey-aligned)" — resolves what looks like a tension between two frameworks and turns it into a design principle. This should be promoted to the main spec or why-bpmux document.

## The Main Pattern (Same as April 1)

Caveats don't always travel with the claim. When the same inference appears in multiple files, the caveat is strong in one file but absent or muted in others. The "coding is mixed" example is the clearest case, but the pattern appears wherever a finding is referenced across documents. A reader encountering a claim for the first time in a secondary document may not see the qualification from the primary one.
