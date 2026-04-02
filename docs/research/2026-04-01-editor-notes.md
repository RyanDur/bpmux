# Editor Notes: Research Verification

*Verified 2026-04-01. 29 claims checked against primary sources.*

**Overall: Strong.** 21 of 29 claims fully confirmed. No fabricated citations. Errors are attribution drift, not substantive misrepresentation.

---

## Must Fix

### 1. Sophie Leroy -- Wrong Term

**File:** `supporting-research.md`
**Problem:** Uses "cognitive residue" -- her term is **"attention residue."**
**Source:** Leroy, S. (2009). "Why is it so hard to do my work?" *Organizational Behavior and Human Decision Processes*, 109(2), 168-181.

### 2. James Bessen -- Wrong Affiliation

**Files:** `inspirations.md`, possibly `abstraction-transitions.md`
**Problem:** Attributed as "Yale/IMF" -- Bessen is at **Boston University School of Law.** The "Yale" confusion likely comes from Yale University Press publishing his book *Learning by Doing*. The "IMF" part refers to the publication venue (*Finance & Development*, March 2015), not his affiliation.
**Fix:** Change to "James Bessen (Boston University, 2015)" or "James Bessen (2015, published in IMF Finance & Development)."

### 3. Kuhl et al. (2022) -- Wrong Authors

**File:** `risk-mitigations.md`
**Problem:** Cited as "Kuhl et al." -- the actual authors are **Fugener, Grahl, Gupta & Ketter.**
**Source:** Fugener, A., Grahl, J., Gupta, A., & Ketter, W. (2022). "Cognitive Challenges in Human-Artificial Intelligence Collaboration: Investigating the Path Toward Productive Delegation." *Information Systems Research*, 33(2).
**Fix:** Replace "Kuhl et al. (2022)" with "Fugener et al. (2022)" throughout.

### 4. Dourish & Bellotti -- Mischaracterized Focus

**File:** `counter-arguments.md`
**Problem:** Characterized as "awareness overload research." Their 1992 CSCW paper ("Awareness and Coordination in Shared Workspaces") is actually about **awareness benefits and mechanisms.** They noted costs to information providers, but the overload framing came from later researchers (Hudson & Smith, 1996; others).
**Fix:** Either reattribute the overload concern to the correct later researchers, or soften to something like "Building on Dourish & Bellotti's awareness framework, later researchers identified awareness overload as a risk."

---

## Minor -- Author's Discretion

### 5. Rubinstein, Meyer & Evans (2001) -- 40% Figure

**File:** `supporting-research.md`
**Context:** The 40% productivity loss claim is from David Meyer's public commentary and interviews, not directly from the paper's experimental data. The paper demonstrated increasing time costs with task complexity but didn't produce a single "40%" figure.
**Suggestion:** Add "estimated" or attribute to Meyer's commentary rather than the paper itself. e.g., "Meyer estimated task switching can cost up to 40% of productive time."

### 6. Nielsen (1995) -- Progressive Disclosure Date

**File:** `awareness-design.md`
**Context:** Progressive disclosure as a concept predates Nielsen -- it appears in 1980s HCI literature. Nielsen's 1994/1995 contribution was the "10 Usability Heuristics," which doesn't specifically name progressive disclosure. He applied and popularized the concept in later work.
**Suggestion:** Could cite as "Nielsen Norman Group" without a specific year, or note "popularized by Nielsen" rather than implying he originated it in 1995.

### 7. Steve Yegge -- Beads Stars and Orchestrator Count

**File:** `multi-agent-landscape.md`
**Context:** "18.7k GitHub stars" was likely accurate when written but Beads has since crossed ~20k. The "four failed orchestrator iterations" framing is slightly imprecise -- Gas Town is the fourth *functioning* orchestrator (v1 TS/Temporal failed, v2 TS failed but produced Beads, v3 Python, v4 Go/Gas Town).
**Suggestion:** Minor. Could qualify stars with "as of [date]" or just round to "~19k." The orchestrator narrative could say "after multiple iterations" rather than specifying a count.

### 8. Frontiers in Psychology (2023) -- Strategy Attribution

**File:** `risk-mitigations.md`
**Context:** The four coping strategies (ignoring, filtering, customizing, saving) are a real taxonomy in information overload literature, but the specific attribution to the 2023 Frontiers paper couldn't be definitively confirmed. The taxonomy may predate or merely be cited by that paper.
**Suggestion:** If the specific paper can be confirmed as the source, keep as-is. Otherwise, soften attribution to "information overload literature" or find the originating paper for those four strategies.

---

## Confirmed Claims (no action needed)

Hannay et al. (2009), Gloria Mark (UC Irvine), Wegner (1987), Gutwin & Greenberg (2002), Vaccaro et al. (2024, Nature Human Behaviour -- including the 370/106 numbers), Bolici et al. (2016), Jevons Paradox (1865), Zach Wills (20 agents, 800 commits, $6k), Google DeepMind (2025, 180 configurations), Lou et al. (2025, arXiv:2504.05755), Peng et al. (2023), METR (2025), Brooks' Law (1975), Weiser & Brown (1995) / Amber Case (2015), Tufte (1983), Pirolli & Card (PARC), Grasse (1959), Craig Reynolds (1986), Tim Harford accountant figures, Backus/FORTRAN (1957) and Hamming quote, Smashing Magazine (2025).

---

## Distillation Review: How Well the Research Is Interpreted

*Beyond citation accuracy -- does the reasoning hold?*

**Overall: The distillation is unusually honest and well-structured.** The epistemic discipline -- explicitly separating "research validates the problem" from "research validates our solution" -- is the strongest quality across all eight files. The caveats are real, not performative. That said, there are five places where the reasoning stretches further than the evidence supports, or where a real tension is underplayed.

### 1. "Coding is a creation task" is doing heavy load-bearing work

**Files:** `counter-arguments.md`, `risk-mitigations.md`, `supporting-research.md`

The Vaccaro et al. meta-analysis found positive effects for "creation tasks" -- but their creation category was text and image generation, not software engineering. The docs acknowledge this caveat, but then lean on "coding is a creation task" heavily across multiple files as a core justification for why bpmux avoids the human-AI underperformance finding.

The problem: coding has a large *decision-making* component (architecture choices, design tradeoffs, bug triage, dependency selection). Decision-making is exactly the category where the meta-analysis found human-AI teams perform *worse*. The caveat is stated once but gets quieter with each repetition across files.

**Suggestion:** Either strengthen the caveat each time this inference appears, or find research that explicitly classifies software engineering within the creation/decision taxonomy. As it stands, the strongest counter-argument in the research (human-AI teams underperform on decisions) is partially neutralized by an unverified classification.

### 2. The stigmergy-solves-Brooks'-Law argument is too clean

**File:** `risk-mitigations.md`

The argument: Brooks' Law describes quadratic communication overhead from pairwise channels. Stigmergic systems use one shared environment, so coordination scales linearly, not quadratically. Therefore bpmux is "inherently resistant" to Brooks' Law.

This is structurally true of the *channel architecture*, but it elides a real cost: **reading the shared log is not free.** As participants increase, the volume of traces grows, and each participant's cost of reading and filtering increases. The docs recognize this as "pheromone oversaturation" in `counter-arguments.md`, but `risk-mitigations.md` presents stigmergy as a structural *solution* to Brooks' Law rather than a *different scaling curve with its own costs*.

**Suggestion:** Reframe as: "Stigmergy replaces quadratic communication overhead with linear reading overhead plus a noise-filtering problem. This is better, but not free. Token awareness and bounded windows are the cost-management mechanism." The current framing ("structural property of the design") overstates the case.

### 3. The Crowston finding is applied more broadly than warranted

**Files:** `inspirations.md`, `abstraction-transitions.md`, `risk-mitigations.md`

Crowston studied FLOSS development -- asynchronous, code-artifact-based coordination over days and weeks. bpmux is real-time, message-based coordination in synchronous pairing sessions. The docs say "bpmux extends Crowston's finding from code artifacts to real-time conversation" -- but that's a significant leap across two dimensions (async → sync, artifact-based → message-based).

Crowston's key finding was that code artifacts *replaced* the need for direct communication. bpmux is *adding* a new communication channel on top of code artifacts. Whether the stigmergic pattern that eliminated communication in async FLOSS also works when applied to a real-time chat channel is an open question, not an extension of a confirmed finding.

**Suggestion:** Reframe as: "Crowston demonstrated that stigmergic coordination works in software development. bpmux hypothesizes that the same principle applies to real-time conversation, not just asynchronous code artifacts. This is an informed design choice, not a validated extension of Crowston's finding."

### 4. The FORTRAN analogy is emotionally resonant but structurally incomplete

**File:** `abstraction-transitions.md`

The parallel is clean: FORTRAN automated assembly, AI automates coding, resistance follows the same emotional arc, outcome will be the same expansion of scope. The doc handles this well overall.

But there's a structural disanalogy worth noting: **FORTRAN's output was deterministic.** The same FORTRAN code compiled to the same machine code every time. Once you verified the compiler was correct, you could trust it. AI outputs are stochastic -- the same prompt produces different code each time, and verification is harder than compilation. The doc says "the developer who understands architecture becomes the most valuable person when things go wrong" -- but doesn't address that things go wrong *more often and less predictably* with stochastic tools than with deterministic compilers.

This doesn't invalidate the analogy. The emotional arc and the scope-expansion pattern are real. But noting the disanalogy would strengthen the piece by showing the author has thought about where the parallel breaks down.

**Suggestion:** Add a paragraph acknowledging the deterministic/stochastic difference, and note that this is why bpmux emphasizes TDD and verification gates -- they compensate for the property that FORTRAN compilers had but AI doesn't.

### 5. Jevons Paradox supports "demand grows" but not "coordination is the next bottleneck"

**Files:** `inspirations.md`, `abstraction-transitions.md`

The Jevons Paradox application is correct: if AI makes software cheaper to produce, demand will grow. The historical examples (accountants, bank tellers) support this well.

But the specific claim -- that *coordination* becomes the binding constraint -- is a separate hypothesis from Jevons. Jevons predicts demand growth. It doesn't predict which constraint becomes binding next. Coordination is one candidate. Quality assurance, security review, regulatory compliance, and product judgment are others. The docs treat "coordination is the next bottleneck" as following from Jevons, but it's actually an independent bet.

**Suggestion:** Separate the two claims. "Jevons predicts more software will be built. Our *additional* hypothesis is that coordination becomes the binding constraint on teams that build it. This is our bet, informed by but not proven by Jevons."

---

### What's Not an Issue

- **The awareness-design synthesis** (`awareness-design.md`) is the strongest file in the set. The five design philosophies are well-chosen, well-sourced, and the synthesis into a concrete rendering model is clean. No notes.
- **The counter-arguments file** is genuinely adversarial and doesn't soften the strongest objections. This is rare and credible.
- **The risk-mitigations separation** of tool features from process discipline is honest and important. The "bpmux provides X, the working agreement provides Y, neither alone is sufficient" framing is exactly right.
- **The multi-agent landscape survey** is comprehensive and the "Key Insight" (solo-multi-agent vs pair-programming-with-AI as different problems) is a genuine contribution to the framing.

### The Main Pattern

The recurring issue across all five points above is: **research that validates the problem is sometimes applied to validate the solution.** The docs are *aware* of this distinction (the caveats exist), but the framing blurs it in places -- especially when the same inference is repeated across multiple files and the caveat doesn't travel with it. A reader who sees "coding is a creation task" for the third time may not remember the caveat from the first time.
