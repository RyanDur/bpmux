# Research Methodology Analysis

An evaluation of bpmux's multi-wave research process against established SE research methodology frameworks.

## Sources Examined

1. **Storey et al., "A Disruptive Research Playbook for Studying Disruptive Innovations"** (ACM TOSEM, 2024) -- The primary framework. Introduces a four-step playbook grounded in McGrath's triadic domain and McLuhan's tetrad for formulating research questions about disruptive technologies.

2. **MyResearchPlaybook** (GitHub, margaretstorey/MyResearchPlaybook) -- A custom GPT implementation of the Disruptive Research Playbook. Operationalizes the paper's framework into an AI-assisted research question generation tool using McLuhan's tetrad and ACM SIGSOFT Empirical Standards.

3. **Schwab & Held, "Ten Simple Rules for Good Research Practice"** (PLOS Computational Biology, 2022) -- Ten rules organized across planning (5 rules), execution (3 rules), and reporting (2 rules) of research. Covers question formulation, study planning, preregistration, data management, and transparent reporting.

4. **ACM SIGSOFT Empirical Standards** (Ralph et al., 2021) -- Method-specific evidence standards for SE research. The General Standard defines essential attributes for any empirical study (clear purpose, appropriate methodology, defined terminology, threats to validity). Eighteen method-specific standards cover everything from controlled experiments to grounded theory.

---

## 1. What Makes a Good Research Question?

### Storey's Playbook

Good research questions emerge from systematic exploration, not intuition. The playbook prescribes:

- **McLuhan's Tetrad** generates questions across four dimensions: What does the technology *enhance*? What does it make *obsolete*? What does it *retrieve* from the past? What does it *reverse into* when pushed to extremes?
- **Matrix construction**: Tetrad dimensions (rows) crossed with phenomena (columns) produce a grid of candidate questions. This prevents tunnel vision on a single angle.
- **Actionability filter**: "Contemplation about the potential impact or actionability of the results" must guide question selection. A question that cannot produce actionable insight is not worth pursuing.
- **Contribution type clarity**: Every question should target one of three contribution types: problem understanding, technology design, or technology evaluation.

### Ten Simple Rules

- Rule 1 (Formulate your research question): A successful study requires a narrow and clear research question. Coming up with a question is not always simple and may take time. The question should be specific enough to be answerable, but significant enough to matter.
- Rule 2 (Plan your study): The research design must flow from the question, not the other way around. Methodology serves the question.

### SIGSOFT General Standard

Essential attributes include: "states a purpose, problem, objective, or research question," "methodology is appropriate (not necessarily optimal) for stated purpose or questions," and "defines all terminology."

### Synthesis for bpmux

A good research question is:
- **Specific** enough to be answerable within scope
- **Actionable** -- the answer changes what you build or how you build it
- **Multi-dimensional** -- explored from multiple angles (not just the obvious one)
- **Contribution-typed** -- you know whether you are understanding a problem, designing a solution, or evaluating one

---

## 2. How Should Research Be Structured (Methodology Selection)?

### McGrath's Triadic Domain

Storey's playbook uses McGrath's framework to connect three interdependent domains:

| Domain | Question | bpmux Equivalent |
|---|---|---|
| **Substantive** | What are you studying? (the phenomena, units of analysis) | The actual problem space: cognitive debt, intent misalignment, multi-agent coordination |
| **Conceptual** | What theories or constructs frame your understanding? | Pair programming principles, message bus architecture, conduit model |
| **Methodological** | How will you gather and analyze evidence? | Literature review, competitive analysis, practitioner interviews, prototype testing |

The key insight: these three domains must be coherent. A mismatch between what you study (substantive), what you think about it (conceptual), and how you investigate it (methodological) produces unreliable results.

### Research Strategy Trade-offs

Storey maps four strategy types from McGrath:

| Strategy | Strength | Weakness |
|---|---|---|
| Laboratory studies (controlled experiments) | Internal validity, control | Low realism |
| Field studies (observation in context) | Ecological validity, realism | Low control |
| Respondent strategies (surveys, interviews) | Generalizability | Low realism |
| Data studies (mining, analytics) | Precision, scale | No human participant involvement |

No single strategy covers all bases. The playbook advocates selecting strategies that "ideally directly involve human participants" for socio-technical phenomena -- which is exactly what bpmux is studying.

### Ten Simple Rules

- Rule 2 (Plan your study): Design follows question. Choose methodology that fits the question, not the methodology you know best.
- Rule 3 (Plan for data sharing and reproducibility): Build reproducibility into the design from the start, not as an afterthought.
- Rule 4 (Preregister your study): Commit to your analysis plan before seeing results. This separates confirmatory from exploratory findings.

### SIGSOFT General Standard

"Methodology is appropriate (not necessarily optimal) for stated purpose or questions." The word "appropriate" is doing heavy lifting -- it requires an intelligent judgment about fit between purpose and method, not blind adherence to a checklist.

---

## 3. The "Who, What, How" Framework for SE Research

McGrath's triadic domain maps directly to Who/What/How:

- **Who** (Substantive Domain): Who are the participants? What phenomena are being studied? In bpmux's case: AI pair programming practitioners, their reasoning processes, their coordination failures.
- **What** (Conceptual Domain): What theoretical lens frames the investigation? For bpmux: cognitive debt theory, pair programming principles (Navigator/Driver), message bus architecture patterns, the conduit model.
- **How** (Methodological Domain): How will evidence be gathered? For bpmux: competitive analysis, practitioner experience, prototype-driven validation, TDD as empirical feedback.

### The Coherence Requirement

The three domains must form a coherent triangle. Storey's retrospective analysis of Stack Overflow research revealed that "much of cited research on Stack Overflow did not consider human and social aspects" -- a failure of Who/What alignment. Researchers studied a social platform (Who: developers asking questions) but framed it through purely technical lenses (What: code quality metrics) using methods that excluded human participants (How: repository mining only).

This is a direct warning for bpmux: if we claim to address cognitive debt in human-AI collaboration, our research must actually study humans collaborating with AI, not just tool features.

---

## 4. How to Evaluate Sources and Avoid Bias

### Storey's Approach

The playbook does not prescribe a formal source evaluation protocol, but embeds bias mitigation structurally:

- **Multi-dimensional questioning** (the tetrad) forces examination of a technology from enhancement, obsolescence, retrieval, and reversal angles. This prevents confirmation bias -- you cannot only study what a technology improves.
- **Interdisciplinary theory integration**: Drawing on social science theories (Technology Acceptance Model, diffusion of innovation, coordination theory) beyond the home discipline prevents disciplinary tunnel vision.
- **Human oversight of AI-generated research**: The paper explicitly notes that MyResearchPlaybook "required significant human oversight and interpretation" to avoid oversimplification.

### Ten Simple Rules

- Rule 5 (Report transparently): Distinguish between planned and unplanned analyses. Report what you found, not what you hoped to find.
- Rule 7 (Be aware of cognitive biases): Confirmation bias, anchoring, availability bias, and hindsight bias all distort research. Awareness is necessary but not sufficient -- structural safeguards (preregistration, blinding, adversarial review) are required.
- Rule 8 (Get feedback early and often): Peer review is not just for finished work. Share drafts, present at seminars, invite critique before you are committed to conclusions.

### SIGSOFT General Standard

Essential: "discusses threats to validity or limitations." Desirable: "discusses alternative explanations for findings." The standard requires researchers to actively argue against their own conclusions.

### Practical Heuristics for Source Evaluation

From these frameworks combined:

1. **Triangulate**: No single source or method is sufficient. Cross-reference claims across independent sources.
2. **Check the tetrad**: Does the source only examine enhancement? What about obsolescence, retrieval, reversal?
3. **Identify the contribution type**: Is the source understanding a problem, proposing a design, or evaluating a solution? Mismatched contribution types cannot be directly compared.
4. **Look for who is missing**: Storey's Stack Overflow finding applies broadly -- whose perspective is absent from this source?
5. **Distinguish evidence from advocacy**: Sources produced by tool vendors or project champions have structural incentive bias.

---

## 5. How to Know When Research Is "Enough" (Diminishing Returns)

None of the four sources provide explicit stopping criteria. This is a known gap in research methodology literature. However, the frameworks collectively suggest:

### Saturation Indicators

- **Tetrad coverage**: Have you examined the technology from all four McLuhan dimensions? If your research only addresses enhancement and obsolescence but not retrieval and reversal, you are not done.
- **Domain coherence**: Can you articulate a coherent Who/What/How triangle? If any vertex is underdeveloped, that is where more work is needed.
- **Question answering**: Can you answer your specific research questions with evidence? If you can state the answer but not support it, you need more evidence. If you can support it but the answer does not change your design, the research was not actionable.

### Diminishing Returns Signals

- **New sources repeat existing findings** without adding new dimensions or contradictions.
- **Cross-referencing produces agreement** rather than new questions.
- **The reversal question is answered**: You can articulate what happens when the technology is pushed to extremes. This is typically the last dimension to resolve because it requires the deepest understanding.

### The Actionability Test

Storey's emphasis on "potential impact or actionability of the results" provides the sharpest stopping criterion: **research is enough when further investigation would not change what you build next**. If the next story, the next design decision, the next architectural choice is already clear -- and you can defend it against the tetrad -- stop researching and start building.

---

## 6. How to Organize and Catalog Findings

### Storey's Matrix Approach

The tetrad-by-phenomena matrix is itself an organizational structure:

|  | Phenomenon A | Phenomenon B | Phenomenon C |
|---|---|---|---|
| **Enhances** | RQ-1A | RQ-1B | RQ-1C |
| **Obsoletes** | RQ-2A | RQ-2B | RQ-2C |
| **Retrieves** | RQ-3A | RQ-3B | RQ-3C |
| **Reverses** | RQ-4A | RQ-4B | RQ-4C |

Each cell contains a research question and its associated evidence. This forces systematic coverage and makes gaps visible.

### Ten Simple Rules

- Rule 6 (Manage your data): Organize data as you collect it, not after. Use consistent naming, version control, and metadata. "Data management is not a post-hoc activity."
- Rule 9 (Document your research process): Keep a research log. Record decisions, dead ends, and changes of direction -- not just results.

### SIGSOFT General Standard

Desirable: "provides supplementary materials such as data, scripts, or extended analyses." The standard treats organized, accessible evidence as a quality marker.

### Practical Catalog Structure

From these frameworks, effective research organization requires:

1. **Question-indexed**: Findings organized by the question they answer, not by source or date.
2. **Evidence-graded**: Each finding tagged with its evidence type (empirical study, expert opinion, architectural pattern, competitive analysis) and strength.
3. **Contradiction-tracked**: Disagreements between sources preserved, not resolved by fiat.
4. **Gap-mapped**: The matrix makes missing cells visible. Empty cells are findings too.

---

## 7. The Iterative Research Process -- How Does Critique Fit In?

### Storey's Position

The playbook is explicitly iterative: "Although we number and present the steps in order, it is important to note that iteration may and often should occur throughout the process." The playbook is "not designed to be prescriptive but rather to aid in an iterative brainstorming process and to help question the choice of research questions and strategies."

Critique serves two functions:
1. **Question refinement**: Critique of research questions (are they actionable? multi-dimensional? properly scoped?) happens before methodology selection.
2. **Design coherence**: Critique of the Who/What/How triangle ensures the three domains remain aligned as understanding evolves.

### Ten Simple Rules

- Rule 8 (Get feedback early and often): The single most important rule for iterative research. "Do not wait until the manuscript is written to seek feedback." Early critique prevents wasted effort on poorly framed questions.
- Rule 10 (Be open to revision): Research conclusions are provisional. New evidence should change your mind. The inability to revise conclusions is a research failure, not a strength.

### The Critique Cycle

Combining all four sources, the healthy critique cycle is:

1. **Formulate** questions using systematic frameworks (tetrad, matrix)
2. **Investigate** using methodology matched to the question
3. **Critique** findings against the tetrad dimensions -- what did you miss?
4. **Revise** questions and methodology based on critique
5. **Repeat** until actionability threshold is met

Critique is not a separate phase -- it is embedded in every transition between steps.

---

## Evaluation of bpmux's Current Research Process

### The Five Waves

| Wave | Current Process | Purpose |
|---|---|---|
| 1 | Raw research (multiple parallel agents) | Breadth: gather diverse perspectives quickly |
| 2 | Cross-referencing critiques (researcher critiques design, designer critiques research) | Adversarial review: find gaps and contradictions |
| 3 | Synthesis into one document | Convergence: unified understanding |
| 4 | Self-critique against founding thesis | Coherence check: does research support or challenge the thesis? |
| 5 | Final distillation | Actionability: extract what matters for building |

### What the Process Does Right

**1. Parallel agents in Wave 1 approximate methodological triangulation.**
Storey's playbook warns against single-method research. By running multiple agents with different perspectives simultaneously, Wave 1 generates the breadth that McLuhan's tetrad demands. Different agents naturally emphasize different dimensions (enhancement vs. obsolescence vs. retrieval vs. reversal) even without being explicitly assigned those lenses.

**2. Cross-referencing critique in Wave 2 is structurally sound.**
This directly implements Rule 8 (get feedback early and often) and Storey's emphasis on "significant human oversight." Having the researcher critique design and the designer critique research mirrors the interdisciplinary tension that produces robust findings. It also maps to the SIGSOFT requirement to "discuss alternative explanations."

**3. Wave 4's thesis-check implements the reversal question.**
By forcing research findings back against the founding thesis, Wave 4 asks: "Does this evidence support what we believed, or does it reveal that our thesis reverses into something we did not intend?" This is McLuhan's fourth dimension applied to self-examination.

**4. The five-wave structure separates divergent from convergent thinking.**
Waves 1-2 are divergent (generate breadth, find contradictions). Waves 3-5 are convergent (synthesize, validate, distill). This separation prevents premature convergence -- a common research failure where the first plausible answer is accepted without exploring alternatives.

**5. Wave 5 enforces the actionability test.**
Distillation forces the question: "What from this research actually changes what we build?" This aligns with Storey's emphasis on "potential impact or actionability of the results."

### What the Process Gets Wrong (or Could Improve)

**1. No explicit question formulation step.**
The most significant gap. Storey's playbook starts with systematic question generation (McLuhan's tetrad, matrix construction, contribution typing). The current process jumps straight to "raw research" without first defining what questions the research must answer. This risks:
- Gathering interesting but irrelevant information
- Missing entire dimensions (the tetrad ensures you examine enhancement, obsolescence, retrieval, and reversal -- skipping this means you might only study the obvious angle)
- No way to assess completeness (you cannot know if research is "enough" without knowing what questions it was supposed to answer)

**Recommendation**: Add a Wave 0 -- Question Formulation. Before any research begins, construct a tetrad matrix for the topic. Define specific, actionable questions. Type each question's expected contribution (problem understanding, design, evaluation). This becomes the scorecard for all subsequent waves.

**2. No explicit methodology selection.**
McGrath's triadic domain requires coherence between substantive, conceptual, and methodological domains. The current process does not ask "How should we investigate this?" before investigating. Parallel agents default to literature review and competitive analysis -- but some questions require field studies, practitioner interviews, or prototype testing. Without methodology selection, the research strategy is accidental rather than deliberate.

**Recommendation**: In Wave 0, after formulating questions, select an appropriate research strategy for each question using McGrath's four strategy types. Some questions may be answerable by data studies (competitive analysis); others require respondent strategies (practitioner interviews) or field studies (prototype observation).

**3. Wave 2 critique lacks a structured framework.**
Cross-referencing is valuable, but "researcher critiques design" is too open-ended. Without a framework, critics default to surface-level objections or personal preferences. Storey's tetrad provides a ready-made critique framework: Does the research address all four dimensions? Does the design account for reversal effects?

**Recommendation**: Structure Wave 2 critique around the tetrad matrix from Wave 0. Each critic evaluates whether the research answers each cell in the matrix. Empty cells become explicit action items, not vague feelings of incompleteness.

**4. No contradiction preservation.**
The current process moves from critique (Wave 2) to synthesis (Wave 3) without explicitly tracking contradictions. Synthesis often resolves disagreements by choosing the more convenient position. Storey's framework treats contradictions as signals -- they indicate that the phenomenon is more complex than initially modeled, or that the research question needs refinement.

**Recommendation**: Between Wave 2 and Wave 3, add a Contradiction Register. Every disagreement between agents, between research and design, between evidence and thesis, is logged with both positions and the evidence supporting each. Wave 3 synthesis must address each registered contradiction explicitly -- either resolving it with evidence or preserving it as an open question.

**5. Wave 4 self-critique is necessary but may be insufficient.**
Critiquing against the founding thesis is one form of validation, but it only tests internal coherence. It does not test against external evidence, alternative theories, or the experiences of practitioners outside the project. The Ten Simple Rules emphasize external feedback (Rule 8) and awareness of cognitive biases (Rule 7). Self-critique alone is vulnerable to shared blind spots -- all agents share the same thesis, so they may share the same biases.

**Recommendation**: Supplement Wave 4 with at least one external validation mechanism. Options include: presenting findings to practitioners not involved in the project, testing against a published framework (like the tetrad), or conducting a structured devil's advocate exercise where one agent is explicitly tasked with falsifying the thesis rather than supporting it.

**6. No research log or decision trail.**
Rule 9 (document your research process) and Rule 6 (manage your data) both emphasize recording decisions, dead ends, and changes of direction. The current process produces outputs (the distilled document) but does not preserve the reasoning trail. When a future wave revisits a topic, there is no record of what was already explored and rejected.

**Recommendation**: Each wave should produce not just findings but a brief decision log: what was explored, what was discarded and why, what questions remain open. This log becomes the input for future research on the same topic and prevents redundant investigation.

### Summary: Gap Analysis

| Aspect | Current Coverage | Gap |
|---|---|---|
| Question formulation | Implicit | No systematic question generation (tetrad, matrix) |
| Methodology selection | Accidental (defaults to literature review) | No deliberate strategy selection per question |
| Multi-dimensional exploration | Partial (parallel agents help) | No explicit tetrad coverage requirement |
| Adversarial critique | Strong (Wave 2 cross-referencing) | Lacks structured framework for critics |
| Contradiction handling | Implicit (resolved in synthesis) | No explicit contradiction register |
| External validation | Absent | Self-critique only; shared blind spots risk |
| Actionability filtering | Strong (Wave 5 distillation) | Well-implemented |
| Research logging | Absent | No decision trail or dead-end documentation |
| Thesis coherence check | Strong (Wave 4) | Could benefit from external falsification |
| Stopping criteria | Implicit | No explicit completeness framework |

### Proposed Revised Process

```
Wave 0: Question Formulation
  - Apply McLuhan's tetrad to the research topic
  - Construct phenomena x tetrad matrix
  - Type each question (problem understanding / design / evaluation)
  - Select research strategy per question (McGrath's four types)

Wave 1: Raw Research (unchanged, but guided by Wave 0 matrix)
  - Multiple parallel agents investigate assigned questions
  - Each agent documents sources, methods, and dead ends

Wave 2: Structured Critique (enhanced)
  - Cross-referencing critique using tetrad matrix as scorecard
  - Explicit contradiction register produced
  - Gap identification against matrix cells

Wave 3: Synthesis (enhanced)
  - Unified document addresses each matrix cell
  - Contradictions explicitly resolved or preserved as open questions
  - Evidence strength graded per finding

Wave 4: Validation (enhanced)
  - Self-critique against founding thesis (retained)
  - External validation: devil's advocate falsification exercise
  - Tetrad reversal check: "What happens if we push this to extremes?"

Wave 5: Final Distillation (unchanged)
  - Extract actionable findings
  - Produce decision log for future reference
  - Identify remaining open questions for next research cycle
```

---

## Sources

- [Storey et al., "A Disruptive Research Playbook for Studying Disruptive Innovations" (ACM TOSEM 2024)](https://dl.acm.org/doi/10.1145/3678172)
- [Storey, MyResearchPlaybook (GitHub)](https://github.com/margaretstorey/MyResearchPlaybook)
- [ArXiv preprint of the Disruptive Research Playbook](https://arxiv.org/abs/2402.13329)
- [Schwab & Held, "Ten Simple Rules for Good Research Practice" (PLOS Computational Biology 2022)](https://journals.plos.org/ploscompbiol/article?id=10.1371/journal.pcbi.1010139)
- [ACM SIGSOFT Empirical Standards](https://www2.sigsoft.org/EmpiricalStandards/)
- [ACM SIGSOFT Empirical Standards (GitHub)](https://github.com/acmsigsoft/EmpiricalStandards)
