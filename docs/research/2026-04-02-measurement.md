# What We Measure and How

*Informed by Miller (2024) on statistical rigor in evaluation, and the SPACE framework for developer productivity.*

---

## Principle

Miller's "Adding Error Bars to Evals" (2024) argues that evaluations are experiments, and the field has a rigor problem — results reported without error bars, conclusions drawn from highest-number-wins comparisons. We should not repeat this with bpmux. If we claim bpmux reduces cognitive or intent debt, we need to say how we measured that and how confident we are.

At the same time: bpmux is a developer tool being dogfooded by 1-2 people, not a research study with 100 participants. We can't do controlled experiments. What we can do is be honest about what we observe, structured in how we capture it, and explicit about what we can and can't conclude.

## What bpmux Can Measure From Its Own Data

These metrics are producible from bpmux's message log and MCP server state. No surveys or external tools needed. They can be computed automatically.

### 1. Context Recovery Time
**What:** When a new AI session starts and calls `read_messages`, how long (in messages or seconds) until it makes its first substantive contribution that builds on prior context?
**Why it matters:** Directly tests whether the persistent log reduces cold-start cost.
**Baseline comparison:** Same codebase, new AI session without bpmux. How long to get productive?
**Caveat:** "Substantive contribution" requires judgment to define. Automate the timestamp measurement, judge the quality manually.

### 2. Message Reference Rate
**What:** Of messages the AI reads via `read_messages`, what fraction does it reference or build on in subsequent messages?
**Why it matters:** If the AI reads 50 messages and never uses any of them, the log isn't providing useful context. High reference rate = useful context. Low rate = noise.
**How to measure:** Content analysis of AI responses after `read_messages` calls. Can be partially automated (does the response mention topics from recent messages?).

### 3. Session Continuity
**What:** How many sessions reference decisions or context from previous sessions?
**Why it matters:** Tests whether the persistent log actually bridges sessions. If every session starts fresh despite the log, persistence isn't working.
**How to measure:** Count cross-session references in AI messages. "As discussed in the previous session..." or references to decisions made before the current session.

### 4. Human Participation Rate
**What:** Fraction of messages from humans vs AIs over time.
**Why it matters:** Trend toward 0% human participation is the commitment deficit materializing. Healthy ratio means humans are engaged.
**How to measure:** Computed directly from sender identity in the log.
**Caveat:** Low human participation might also mean the AIs are handling things well and the human is appropriately supervising without needing to intervene. Context matters — the number alone is ambiguous.

### 5. Read Cursor Patterns
**What:** How AIs call `read_messages` — defaults (unread only), explicit window (`last: N`), or full history (`all`).
**Why it matters:** Reveals whether the cursor-based approach works. If AIs constantly request `all` or large windows, the cursor isn't providing enough context by default.
**How to measure:** Log the parameters of every `read_messages` call.

### 6. Trust Level Changes
**What:** How often humans adjust trust levels during a session.
**Why it matters:** Frequent changes suggest defaults are wrong. Zero changes could mean defaults are right or nobody's paying attention.
**How to measure:** Log every `/trust` and `/untrust` command.

## What We Observe During Dogfooding (Structured Qualitative)

These require human judgment but should be captured systematically — in the bpmux log itself, or in session retrospective notes.

### 7. Cognitive Debt Test
**After each significant session, answer three questions:**
1. Can you explain why the code is the way it is *without* reading the bpmux log? (If yes: shared understanding maintained)
2. Does the bpmux log contain the reasoning? (If yes: intent captured)
3. Gap between 1 and 2: if the log has it but you don't, intent was captured but cognitive debt still accumulated (you relied on the log instead of understanding).

**Record:** Date, session summary, answers to all three questions.

### 8. The New Session Test
**Periodically, start a fresh AI session with access to the bpmux log and give it a task in the codebase.**
- Does it make reasonable decisions?
- Does it understand prior constraints and decisions?
- Compare to: a fresh session with no bpmux log (if feasible).

**Record:** What the AI got right, what it got wrong, whether the log contained the information it needed.

### 9. Decision Quality
**When the AI surfaces a decision through the chat, track:**
- Did the human engage with it (asked questions, pushed back, modified)?
- Did the human rubber-stamp it (accepted without examination)?
- Did the human ignore it?

**Record:** Decision topic, human response type, outcome if known later.

## What We Don't Measure

Being honest about what we can't claim from n=1-2 dogfooding:

- **Developer satisfaction** — too subjective, no control group
- **Productivity increase** — no baseline, too many confounds, and "productivity" is poorly defined (per SPACE framework, it's multi-dimensional)
- **Code quality improvement** — can't attribute to bpmux vs other factors
- **Statistical significance** — with 1-2 users, nothing reaches significance. We can observe patterns, not prove effects.

## How We Report What We Find

Following Miller's spirit of rigor even at small scale:

1. **State the observation, not the conclusion.** "In 8 of 10 new sessions, the AI referenced prior decisions from the log within its first 3 messages" — not "bpmux eliminates cold-start cost."
2. **Report the denominator.** "5 out of 12 sessions showed cross-session continuity" — not "sessions show continuity."
3. **Note when we can't distinguish explanations.** "Human participation dropped to 15% — this could be disengagement or appropriate delegation. We can't tell from the metric alone."
4. **Capture the qualitative context alongside the quantitative data.** The bpmux log itself is the context record. Retrospective notes go in the session log.

## Sources

- [Miller (2024) — Adding Error Bars to Evals](https://arxiv.org/abs/2411.00640)
- [SPACE Framework — Forsgren et al., ACM Queue](https://queue.acm.org/detail.cfm?id=3454124)
- [Measuring Shared Understanding in Software Teams — ESEM 2014](https://dl.acm.org/doi/10.1145/2652524.2652543)
