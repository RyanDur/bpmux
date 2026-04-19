# Accuracy Audit: Original Research Docs vs. Deep-Read Primary Sources

*April 4, 2026. Systematic check of 93 research documents against deep reads of Clark & Brennan (1991), Gutwin & Greenberg (2002), and Edmondson (1999).*

---

## Methodology

Three deep-read analyses were treated as ground truth:
- `foundations/clark-brennan-deep-read.md` (Clark & Brennan 1991)
- `foundations/gutwin-greenberg-deep-read.md` (Gutwin & Greenberg 2002)
- `foundations/edmondson-deep-read.md` (Edmondson 1999)

All files in `foundations/`, `architecture/`, `risks/`, `landscape/`, `implementation/`, `process/`, `synthesis/`, and `editorial/` were searched for references to these papers, plus Storey (cognitive/intent debt) and ChatCollab (authorship check).

---

## 1. ChatCollab Attribution Error

### Files affected

| File | What it says |
|---|---|
| `synthesis/archive/product-direction-v1.md` (line 37, 303) | "Arawjo et al. (2024)" — file archived 2026-04-19; v1 fix no longer required |
| `synthesis/product-direction-v2.md` (line 375) | "Arawjo et al., 2024" — still current, fix pending |

### The error

ChatCollab (arXiv:2412.01992) is attributed to "Arawjo et al." across all references. The paper's actual first author is Benjamin Klieger. The correct citation is **Klieger et al. (2024)**.

The `process/design-principles.md` file (line 176) references ChatCollab without an author name, linking only to the arXiv URL, which avoids the error.

**Type:** Attribution error
**Severity:** High. Wrong first author on a cited paper is a credibility problem in any research context.

---

## 2. Edmondson: "Transparency creates safety" -- Causal Direction Reversed

### Files affected

| File | What it says |
|---|---|
| `foundations/2026-04-02-collaboration-first.md` (line 35) | "Edmondson's work on psychological safety... shows that high-performing teams are built on the ability to speak up without fear. Transparency in decision-making creates this safety because it removes the asymmetry" |
| `foundations/2026-04-02-developer-psychology.md` (line 44) | "Edmondson's framework supports this -- visible reasoning frames work as a learning problem, not a performance evaluation." |

### What the primary source actually says

Edmondson (1999) does not claim that transparency creates psychological safety. The causal chain is the reverse: **psychological safety enables transparency (learning behaviors including sharing information, admitting errors, seeking feedback).** Safety is the antecedent; transparent behavior is the consequence.

The deep read states: "the relationship between structural features (like transparent information sharing) and performance is fully mediated by psychological safety. Context support (information, resources) predicts safety, and safety predicts learning behavior."

The original docs present transparency as creating safety ("Transparency in decision-making creates this safety"). Edmondson's model says safety creates the willingness to be transparent. The nuance matters: context support (providing information and resources) is *positively associated* with safety, but the mechanism is that resources reduce "insecurity and defensiveness" -- not that transparency per se produces safety.

**Type:** Factual error -- causal direction reversed
**Severity:** High. This changes how bpmux's value proposition should be framed. If transparency creates safety, building a transparent tool is sufficient. If safety enables transparency, the tool alone is insufficient without the interpersonal conditions Edmondson describes.

---

## 3. Edmondson: Missing the Full Mediation Finding

### Files affected

| File | What it says |
|---|---|
| `synthesis/thesis-v2.md` (line 65) | "Psychological safety determines whether any of it works." |
| `foundations/2026-04-02-collaboration-first.md` (line 35-37) | Describes Edmondson's research as connecting transparency to team outcomes directly |

### What the primary source actually says

The deep read identifies a critical structural finding: **safety does not directly improve performance.** It enables learning behaviors, and learning behaviors improve performance. When learning behavior and safety are entered together as predictors, safety becomes insignificant (B = .25, p = .42) while learning stays significant (B = .60, p < .05).

The original docs generally say "psychological safety matters for performance" without noting the full mediation through learning behavior. The thesis-v2 gets closest ("Psychological safety determines whether any of it works") but still elides the mechanism. The precise claim is: safety enables the learning behaviors that produce performance. Safety without learning behavior does not help. Learning behavior without safety does not happen.

**Type:** Missing nuance
**Severity:** Medium. The practical implication is subtle but real: bpmux should be designed to support specific learning behaviors (seeking feedback, discussing errors, testing assumptions), not just "psychological safety" as an abstract prerequisite.

---

## 4. Edmondson: "Not cohesiveness" Distinction Missing

### Files affected

| File | What it says |
|---|---|
| `foundations/2026-04-02-developer-psychology.md` (line 38) | Defines psychological safety as "the belief that one will not be punished or humiliated for speaking up with ideas, questions, concerns, or mistakes" |
| `foundations/2026-04-02-collaboration-first.md` (line 35) | "high-performing teams are built on the ability to speak up without fear" |

### What the primary source actually says

The deep read identifies a key distinction that Edmondson makes explicitly: **psychological safety is not cohesiveness.** "Cohesive groups can suppress disagreement (groupthink). Psychological safety is the opposite: it enables disagreement."

None of the original docs mention this distinction. For bpmux, this matters because a "safe" channel is not one where everyone agrees -- it is one where disagreement is surfaced. If the bpmux log shows only consensus, that could indicate groupthink (low safety masquerading as harmony), not genuine psychological safety.

**Type:** Missing nuance
**Severity:** Medium. Changes how "safety" should be evaluated in bpmux sessions.

---

## 5. Edmondson: Team Efficacy Null Result Missing

### Files affected

All files that reference Edmondson treat psychological safety as important but do not mention the striking null result: **team efficacy (confidence the team can do its work) does not predict learning behavior when controlling for psychological safety.**

### What the primary source actually says

The deep read states: "Believing the team is competent is not enough. Believing it is safe is what matters." This null result strengthens the case for safety as the key variable but is absent from all original docs.

**Type:** Missing nuance
**Severity:** Low. Strengthens existing claims rather than contradicting them.

---

## 6. Clark & Brennan: "Eight constraints" vs. Simplified References

### Files affected

| File | What it says |
|---|---|
| `architecture/2026-04-02-new-inspirations.md` (line 67-73) | "Communication is a collaborative process of establishing common ground. Mere transmission is inadequate... Grounding costs vary by medium. bpmux changes the grounding cost profile: without it, grounding between two AIs has infinite cost. With it, the cost drops but is text-mediated (no co-presence, no gestures)." |

### What the primary source actually says

The claim that "grounding between two AIs has infinite cost" without bpmux is not a Clark & Brennan finding. Clark & Brennan's framework is about human communication. The paper does not discuss AI agents at all (it was written in 1991). The extension to AI agents is the research team's own inference, but it is presented alongside the Clark & Brennan citation as if it follows from the paper.

Additionally, the deep read identifies that bpmux's constraint profile for AI participants is **identical to email** (only reviewability and revisability), not "text-mediated" in some generic sense. The original doc's description ("no co-presence, no gestures") captures only 2 of the 6 absent constraints. The deep read identifies the full picture: no copresence, no visibility, no audibility, no cotemporality, no simultaneity -- with only sequentiality, reviewability, and revisability present.

**Type:** Missing nuance + extrapolation beyond source
**Severity:** Medium. The "infinite cost" claim is unsupported by Clark & Brennan and should be flagged as the team's own inference.

---

## 7. Clark & Brennan: Cotemporality Asymmetry Not Captured in Earlier Docs

### Files affected

| File | What it says |
|---|---|
| `process/design-principles.md` (line 11) | "Text chat is strong on reviewability and sequentiality but completely lacks copresence, visibility, and audibility." |
| `foundations/2026-04-01-supporting-research.md` | Does not reference Clark & Brennan at all |

### What the primary source actually says

The deep read identifies a critical asymmetry: **bpmux has different constraint profiles for human TUI users vs. AI participants.** Humans get cotemporality (via WebSocket). AI agents do not (they poll). This means bpmux is "terminal teleconferencing" for humans but "email" for AIs.

The design-principles doc (written before the deep read) treats bpmux's constraint profile as uniform. It says text chat "completely lacks copresence, visibility, and audibility" -- which is correct -- but does not note the cotemporality split, which the deep read identifies as "one of bpmux's most significant grounding constraints."

**Type:** Missing nuance
**Severity:** Medium. The asymmetry has direct design implications (push vs. poll for AI participants).

---

## 8. Clark & Brennan: "Principle of Least Collaborative Effort" Missing from All Original Docs

### Files affected

No original doc references the principle of least collaborative effort. The concept appears only in the deep read.

### What the primary source actually says

Clark & Brennan replace the "principle of least effort" (minimize speaker effort) with "principle of least collaborative effort" (minimize total effort of all participants). This has direct implications for how AI agents should communicate: front-load effort when repair costs are high, but do not over-formulate when the listener can cheaply fill in gaps.

**Type:** Missing nuance
**Severity:** Medium. This principle directly informs message design conventions for AI agents in bpmux.

---

## 9. Clark & Brennan: The Four States of Understanding Missing

### Files affected

No original doc references Clark & Brennan's four states (State 0: did not notice; State 1: noticed but could not parse; State 2: parsed but did not understand; State 3: understood).

### What the primary source actually says

These four states provide a vocabulary for what "read receipts" and "acknowledgment signals" are actually measuring. The backlog-priorities doc (items 20, 30) discusses grounding mechanisms and read cursors but without this vocabulary. The deep read notes: "No participant can tell whether another participant is in State 0 (never saw the message), State 1 (saw but could not parse), State 2 (parsed but did not understand), or State 3 (understood)."

**Type:** Missing nuance
**Severity:** Low. Useful vocabulary for design discussion but does not change conclusions.

---

## 10. Gutwin & Greenberg: "Almost mandatory" Quote Not Verified

### Files affected

| File | What it says |
|---|---|
| `synthesis/product-direction-v2.md` (line 99) | "Called 'almost mandatory for real-time collaboration.'" |
| `synthesis/backlog-priorities.md` (line 128) | "The researcher called this 'almost mandatory for real-time collaboration.'" |

### What the primary source actually says

The deep read does not contain the phrase "almost mandatory for real-time collaboration." This phrase appears to be the researcher's characterization of Gutwin & Greenberg's findings, not a direct quote from the paper. The backlog-priorities doc correctly attributes it to "the researcher" (not to Gutwin & Greenberg), but product-direction-v2 presents it ambiguously -- it could be read as a quote from the paper.

**Type:** Potential misattribution
**Severity:** Low. The characterization is reasonable but should not appear in quotation marks as if from the paper.

---

## 11. Gutwin & Greenberg: Framework Is Descriptive, Not Prescriptive

### Files affected

| File | What it says |
|---|---|
| `synthesis/backlog-priorities.md` (various) | Uses Gutwin & Greenberg's framework as prescriptive requirements (e.g., "Gap 1", "Gap 2" implying things that must be built) |
| `synthesis/product-direction-v2.md` (various) | Frames awareness elements as features to implement |

### What the primary source actually says

The deep read explicitly notes: "The authors explicitly state they 'do not give prescriptive rules and guidelines, however, since each groupware application will have to operate within particular awareness requirements dictated by the task and the group situation.' The framework is a structured collection of knowledge for iterative design."

The original docs treat the framework as a checklist of requirements. The paper itself says it is an analysis tool. Each gap identified should be evaluated against bpmux's specific task and group situation, not assumed to be a deficiency.

**Type:** Missing nuance
**Severity:** Medium. Changes how the "gaps" should be interpreted in product planning. Not every gap is necessarily a problem for bpmux's specific use case.

---

## 12. Gutwin & Greenberg: Exclusion of Future-Tense Elements

### Files affected

None of the original docs note that Gutwin & Greenberg explicitly exclude future-tense awareness elements from their framework.

### What the primary source actually says

The deep read states: "The paper explicitly excludes future-tense elements from the framework, because 'past and present information can be determined from raw perceptual information, whereas belief about the future involves inference, extrapolation, and prediction.' Designers cannot directly support those."

Several original docs discuss intention signaling and anticipation as Gutwin & Greenberg concepts, which is correct (Intention is a present-tense element: "What goal is that action part of?"). However, any reference to predictive or future-state features should acknowledge that the framework does not cover them.

**Type:** Missing nuance
**Severity:** Low. Does not affect current design decisions significantly.

---

## 13. Storey: "Triple Debt Model" Naming

### Files affected

| File | What it says |
|---|---|
| `foundations/2026-04-02-cognitive-intent-debt.md` (line 7) | "The Triple Debt Model (Storey, 2026)" |

### Accuracy check

The document references Storey's paper (arXiv:2603.22106) and calls it "the Triple Debt Model." No deep read was performed for Storey, so this audit cannot verify whether Storey herself uses the phrase "Triple Debt Model" or whether it was coined by the research team. The distillation-v3 correctly notes this is "a one-month-old preprint" that "has not been empirically tested or independently replicated."

**Type:** Possible coined term attributed to source
**Severity:** Low. Flagged for verification when a Storey deep read is conducted.

---

## 14. Edmondson: "Transparency without trust is surveillance" -- Not an Edmondson Quote

### Files affected

| File | What it says |
|---|---|
| `synthesis/thesis-v2.md` (line 45) | "bpmux requires psychological safety to function. Transparency without trust is surveillance." |
| `foundations/2026-04-02-collaboration-first.md` (line 43) | "Transparency without trust becomes surveillance." |
| `risks/2026-04-02-responsible-ai.md` (line 18) | Discusses the surveillance risk citing psychological safety |

### What the primary source actually says

The deep read's evaluation (Section 7) states the claim is "supported by the paper's findings on threat and visibility" but the phrase "transparency without trust is surveillance" does not appear in Edmondson (1999). It is the research team's synthesis. This is a valid inference from the paper, but it is presented in several docs as if it is an established finding rather than an interpretation.

The deep read's nuance is important: Edmondson also finds that context support (including access to information) is *positively associated* with psychological safety. The relationship is not "safety first, then transparency" but rather "transparency and safety co-develop through early interactions."

**Type:** Missing nuance -- valid inference presented as established finding
**Severity:** Medium. The co-development framing is more accurate and more useful for bpmux design than the sequential prerequisite framing.

---

## 15. Edmondson: Self-Sealing Pattern Not Referenced in Design Docs

### Files affected

The self-sealing pattern (low-safety teams cannot self-correct; high-safety teams can overcome structural obstacles) appears only in the deep read, not in any original doc.

### What the primary source actually says

"Low-learning teams appeared vulnerable to a self-sealing pattern of members having private concerns about the team environment, which led to withholding relevant thoughts and actions and made it difficult to escape the low-learning condition."

This is directly relevant to bpmux: if a channel starts with low safety, more transparency will not fix it. The original docs discuss the importance of early interactions but do not name or describe the self-sealing mechanism.

**Type:** Missing nuance
**Severity:** Medium. Has design implications for onboarding and session initialization.

---

## Summary Table

| # | Issue | Type | Severity | Files Affected |
|---|---|---|---|---|
| 1 | ChatCollab attributed to "Arawjo et al." instead of Klieger et al. | Attribution error | **High** | product-direction-v1, product-direction-v2 |
| 2 | Edmondson: transparency creates safety (causal direction reversed) | Factual error | **High** | collaboration-first, developer-psychology |
| 3 | Edmondson: full mediation through learning behavior not stated | Missing nuance | Medium | thesis-v2, collaboration-first |
| 4 | Edmondson: safety vs. cohesiveness distinction missing | Missing nuance | Medium | developer-psychology, collaboration-first |
| 5 | Edmondson: team efficacy null result missing | Missing nuance | Low | all Edmondson-citing files |
| 6 | Clark & Brennan: "infinite grounding cost" is team inference, not paper finding | Extrapolation beyond source | Medium | new-inspirations |
| 7 | Clark & Brennan: cotemporality asymmetry (human vs. AI) missing | Missing nuance | Medium | design-principles |
| 8 | Clark & Brennan: principle of least collaborative effort missing | Missing nuance | Medium | no original doc |
| 9 | Clark & Brennan: four states of understanding missing | Missing nuance | Low | no original doc |
| 10 | Gutwin & Greenberg: "almost mandatory" is researcher characterization, not paper quote | Potential misattribution | Low | product-direction-v2 |
| 11 | Gutwin & Greenberg: framework is descriptive, not prescriptive | Missing nuance | Medium | backlog-priorities, product-direction-v2 |
| 12 | Gutwin & Greenberg: exclusion of future-tense elements not noted | Missing nuance | Low | no original doc |
| 13 | Storey: "Triple Debt Model" may be coined term | Possible coined term | Low | cognitive-intent-debt |
| 14 | "Transparency without trust is surveillance" is team synthesis, not Edmondson | Missing nuance | Medium | thesis-v2, collaboration-first, responsible-ai |
| 15 | Edmondson: self-sealing pattern not in design docs | Missing nuance | Medium | no original doc |

---

## Corrections Priority

### Must fix (high severity)

1. **ChatCollab authorship:** Change "Arawjo et al." to "Klieger et al." in product-direction-v2.md (line 375). The v1 file has been archived (2026-04-19) to `synthesis/archive/product-direction-v1.md`; no fix required there.

2. **Edmondson causal direction:** Revise collaboration-first.md and developer-psychology.md to state that safety enables transparent behavior, not that transparency creates safety. The relationship is: context support (including transparency infrastructure) is positively associated with safety, and safety enables learning behaviors (including transparent sharing). The tool provides context support; it does not create safety directly.

### Should fix (medium severity)

3. Clarify in thesis-v2 that safety's effect on performance is fully mediated by learning behavior.
4. Add the safety-is-not-cohesiveness distinction to the developer-psychology doc.
5. Flag the "infinite grounding cost" claim in new-inspirations.md as team inference.
6. Add the cotemporality asymmetry (human vs. AI) to design-principles.md.
7. Note that Gutwin & Greenberg's framework is descriptive, not prescriptive, in backlog-priorities.md.
8. Reframe "transparency without trust is surveillance" as team synthesis supported by Edmondson, not as Edmondson's finding.
9. Add the self-sealing pattern to onboarding/design rationale.
10. Add the principle of least collaborative effort to the design principles or grounding discussion.

### Nice to have (low severity)

11. Remove quotation marks from "almost mandatory" or attribute it to the researcher explicitly.
12. Add the four states of understanding vocabulary to grounding mechanism discussions.
13. Note Gutwin & Greenberg's exclusion of future-tense elements.
14. Verify whether Storey uses "Triple Debt Model" when a deep read is conducted.
15. Add the team efficacy null result as supporting evidence.
