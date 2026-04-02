# Editor Notes: Research Verification

*Originally verified 2026-04-01. 29 claims checked against primary sources.*
*Second verification 2026-04-01. Must-fix items confirmed applied; 5 new issues found.*

**Overall: Strong.** 21 of 29 original claims fully confirmed. No fabricated citations. All four must-fix issues from the first review have been correctly applied. Five new minor issues identified in the second pass.

---

## Must Fix (First Review) -- ALL RESOLVED

### 1. Sophie Leroy -- Wrong Term ✅

**File:** `supporting-research.md`
**Problem:** Used "cognitive residue" -- her term is **"attention residue."**
**Status:** Fixed. File now correctly uses "attention residue."

### 2. James Bessen -- Wrong Affiliation ✅

**Files:** `inspirations.md`
**Problem:** Was attributed as "Yale/IMF" -- Bessen is at **Boston University School of Law.**
**Status:** Fixed. Now reads "James Bessen (Boston University, 2015; published in IMF *Finance & Development*)."

### 3. Kuhl et al. (2022) -- Wrong Authors ✅

**File:** `risk-mitigations.md`
**Problem:** Was cited as "Kuhl et al." -- the actual authors are **Fugener, Grahl, Gupta & Ketter.**
**Status:** Fixed. Now uses "Fugener, Grahl, Gupta & Ketter" and "Fugener et al." throughout.

### 4. Dourish & Bellotti -- Mischaracterized Focus ✅

**File:** `counter-arguments.md`
**Problem:** Was characterized as "awareness overload research." Their 1992 paper is about awareness benefits and mechanisms.
**Status:** Fixed. Now reads "Building on their awareness framework, later CSCW researchers (Hudson & Smith 1996, and the broader information overload literature) identified..."

---

## Should Fix (Second Review) -- NEW

### 9. Weiser & Brown -- Wrong Date

**File:** `awareness-design.md`
**Problem:** Section header says "Weiser & Brown 1995" -- the paper ("The Coming Age of Calm Technology") was published **October 5, 1996**, not 1995. Other files correctly use 1996.
**Fix:** Change "Weiser & Brown 1995" to "Weiser & Brown 1996" in the section header.

### 10. Pierre-Paul Grassé -- Imprecise Date

**File:** `inspirations.md`
**Problem:** Says stigmergy was "coined by Pierre-Paul Grassé in the 1950s" -- the correct year is **1959**. The editor notes' own confirmed claims list says "Grasse (1959)."
**Fix:** Change "in the 1950s" to "in 1959" for consistency across the docs.

### 11. Bessen -- Teller Employment Upper Bound

**File:** `inspirations.md`
**Problem:** Claims teller employment grew "from ~500,000 to ~600,000." Verification against Bessen's original data suggests the upper bound is closer to **~550,000**, not ~600,000. The 43% branch increase figure is confirmed.
**Fix:** Change "~600,000" to "~550,000" or soften to "from ~500,000 to over ~550,000."

---

## Minor -- Author's Discretion

### 5. Rubinstein, Meyer & Evans (2001) -- 40% Figure ✅

**File:** `supporting-research.md`
**Context:** The 40% figure is from Meyer's public commentary, not the paper.
**Status:** Fixed. Now reads "Meyer estimated in public commentary that task switching can cost as much as 40% of productive time — this figure is from his interviews, not directly from the paper's experimental data."

### 6. Nielsen (1995) -- Progressive Disclosure Date

**File:** `awareness-design.md`
**Context:** Progressive disclosure predates Nielsen -- appears in 1980s HCI literature. Nielsen popularized the concept but didn't originate it.
**Status:** The heading now reads "popularized by Nielsen Norman Group" which is accurate. No further action needed.

### 7. Steve Yegge -- Beads Stars and Orchestrator Count

**File:** `multi-agent-landscape.md`
**Context:** Star count was accurate when written but changes over time. Orchestrator iteration count is slightly imprecise.
**Status:** Updated to "~19k+" and "after multiple iterations." Adequate.

### 8. Frontiers in Psychology (2023) -- Strategy Attribution

**File:** `risk-mitigations.md`
**Context:** The four coping strategies taxonomy may predate the 2023 Frontiers paper.
**Status:** `risk-mitigations.md` now hedges with "taxonomy cited in a 2023 *Frontiers in Psychology* review, though the taxonomy may originate from earlier work." Adequate.

### 12. Sophie Leroy -- Affiliation Precision (NEW)

**File:** `supporting-research.md`
**Context:** Listed as "University of Washington" -- she is at **UW Bothell** specifically. At the time of the 2009 paper, she may have been at the University of Minnesota Carlson School.
**Suggestion:** Minor. Could specify "UW Bothell" for precision, but "University of Washington" is not wrong.

### 13. Gloria Mark -- "3 Minutes" Precision (NEW)

**File:** `supporting-research.md`
**Context:** Her research found workers spend an average of **3 minutes and 5 seconds** on a single event before switching. "3 minutes" is a reasonable approximation but not exact.
**Suggestion:** Trivial. Could say "about 3 minutes" for accuracy. Current phrasing ("every 3 minutes on average") is close enough.

---

## Confirmed Claims (no action needed)

Hannay et al. (2009), Gloria Mark (UC Irvine), Wegner (1987), Gutwin & Greenberg (2002), Vaccaro et al. (2024, Nature Human Behaviour -- including the 370/106 numbers), Bolici et al. (2016), Jevons Paradox (1865), Zach Wills (20 agents, 800 commits, $6k), Google DeepMind (2025, 180 configurations), Lou et al. (2025, arXiv:2504.05755), Peng et al. (2023), METR (2025), Brooks' Law (1975), Amber Case (2015), Tufte (1983), Pirolli & Card (PARC), Craig Reynolds (1986), Tim Harford accountant figures, Backus/FORTRAN (1957) and Hamming quote, Smashing Magazine (2025), Rubinstein/Meyer/Evans (2001), Fugener et al. (2022, ISR).

**Moved to "Should Fix":** Weiser & Brown (date is 1996, not 1995), Grassé (date is 1959, not "1950s"), Bessen teller figures (~550k, not ~600k).

---

## Distillation Review: How Well the Research Is Interpreted

*Beyond citation accuracy -- does the reasoning hold?*

**Overall: The distillation is unusually honest and well-structured.** The epistemic discipline -- explicitly separating "research validates the problem" from "research validates our solution" -- is the strongest quality across all eight files. The caveats are real, not performative. The second review found that 3 of 5 original distillation concerns have been fully addressed, 2 partially addressed.

### 1. "Coding is a creation task" is doing heavy load-bearing work — PARTIALLY ADDRESSED

**Files:** `counter-arguments.md`, `risk-mitigations.md`, `supporting-research.md`

The Vaccaro et al. meta-analysis found positive effects for "creation tasks" -- but their creation category was text and image generation, not software engineering. The docs acknowledge this caveat, but then lean on "coding is a creation task" heavily across multiple files.

**Second review:** `risk-mitigations.md` now explicitly says "coding is inherently mixed" and "We cannot lean on 'coding is a creation task' as a blanket justification." This is the strongest treatment. `supporting-research.md` also notes the caveat. However, `counter-arguments.md` still uses the creation-task classification without repeating the caveat. A reader encountering it there for the first time may not know it's contested.

**Remaining suggestion:** Add the "coding is mixed" caveat to `counter-arguments.md` where the creation-task classification appears.

### 2. The stigmergy-solves-Brooks'-Law argument is too clean — ADDRESSED ✅

**File:** `risk-mitigations.md`

**Second review:** Now reads: "Stigmergy replaces quadratic overhead with linear reading overhead plus a noise-filtering problem. This is better, but not free. Token awareness and bounded windows are the cost-management mechanism." The summary table still says "Structural property of the design" but the body text is honest. Adequate.

### 3. The Crowston finding is applied more broadly than warranted — ADDRESSED ✅

**Files:** `inspirations.md`, `abstraction-transitions.md`

**Second review:** Both files now include the caveat that extending Crowston from async FLOSS to real-time conversation is "a leap across two dimensions (async → sync, code artifacts → messages) that has not been validated." `inspirations.md` explicitly says "This is an informed design choice based on Crowston's work, not a validated extension of it." Well handled.

### 4. The FORTRAN analogy -- deterministic vs stochastic gap — ADDRESSED ✅

**File:** `abstraction-transitions.md`

**Second review:** Now includes a full paragraph: "Where the analogy breaks down: FORTRAN's output was deterministic... AI outputs are stochastic... This is why bpmux's spec emphasizes TDD and verification gates — they compensate for the property that FORTRAN compilers had but AI tools don't." Exactly what was recommended.

### 5. Jevons Paradox supports "demand grows" but not "coordination is the next bottleneck" — PARTIALLY ADDRESSED

**Files:** `inspirations.md`, `abstraction-transitions.md`

**Second review:** `inspirations.md` now says "Our hypothesis: if AI makes individual development more efficient, coordination becomes the new bottleneck. bpmux is a bet on this hypothesis." The word "hypothesis" helps. But the two claims (Jevons predicts demand; coordination is the bottleneck) are still presented in the same paragraph, making them feel like one argument rather than a research finding + an independent bet.

**Remaining suggestion:** Separate into two sentences or a brief note: "Jevons predicts more software will be built. Our *additional* hypothesis is that coordination becomes the binding constraint. This is our bet, informed by but not proven by Jevons."

---

### What's Not an Issue

- **The awareness-design synthesis** (`awareness-design.md`) is the strongest file in the set. The five design philosophies are well-chosen, well-sourced, and the synthesis into a concrete rendering model is clean. No notes.
- **The counter-arguments file** is genuinely adversarial and doesn't soften the strongest objections. This is rare and credible.
- **The risk-mitigations separation** of tool features from process discipline is honest and important. The "bpmux provides X, the working agreement provides Y, neither alone is sufficient" framing is exactly right.
- **The multi-agent landscape survey** is comprehensive and the "Key Insight" (solo-multi-agent vs pair-programming-with-AI as different problems) is a genuine contribution to the framing.

### The Main Pattern

The recurring issue across the two remaining partially-addressed points is: **caveats don't always travel with the claim.** When the same inference appears in multiple files, the caveat is strong in one file but absent or muted in others. A reader who encounters "coding is a creation task" in `counter-arguments.md` first may not see the "coding is mixed" qualification from `risk-mitigations.md`.
