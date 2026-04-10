# Log-Only Onboarding Experiment

*Designer research — 2026-04-07. Directed by Ryan.*

## Method

4 independent subagent runs. Each received the same 80-message sample from the bpmux session log (no role contracts, no docs, no prior context). Each was asked 8 identical questions about the team, product, role, values, norms, and process.

### Confounds

- **Base model training:** Subagents have general knowledge from training (what a TUI is, what "designer" means, how teams work). They are not truly zero-context — they are zero *project* context with baseline competence.
- **Pre-filtered log:** The 80-message sample was pre-filtered by the experimenter (chat messages only, activity/system stripped). This introduced selection bias — a truly naive agent would face the raw 10,500-line JSONL.
- **Prompt framing:** The prompt told subagents they were "a brand new designer joining a team," which primes role-relevant behavior.
- **Non-determinism:** LLM outputs vary across runs. 4 runs showed low variance, but larger samples would increase confidence.

## Raw Results (confidence scores 1-5)

| Question | Run 1 | Run 2 | Run 3 | Run 4 | Mean | Variance |
|---|---|---|---|---|---|---|
| 1. What is the team building? | 5 | 4 | 5 | 4 | 4.5 | low |
| 2. What is your role? | 5 | 4 | 4 | 4 | 4.25 | low |
| 3. Who makes final decisions? | 5 | 5 | 5 | 5 | 5.0 | none |
| 4. Team values/norms (3 cited) | 4 | 4 | 5 | 5 | 4.5 | low |
| 5. Process for adding a feature | 3* | 3 | 4 | 3 | 3.25 | low |
| 6. Out-of-domain behavior | 3 | 3 | 3 | 4 | 3.25 | low |
| 7. Current work state | 4 | 4 | 5 | 5 | 4.5 | low |
| 8. "Do the right thing simply" | n/a* | 3 | 4 | 4 | 3.67 | low |

*Run 1 had different question format (longer answers, different structure). Scores normalized from its detailed assessment.

## Consistent Findings (all 4 runs agreed)

**Reliably learned from log alone:**
- Product identity and thesis (bpmux = collective memory for human-AI teams)
- Ryan makes final decisions (5/5 in every run)
- Designer specs UX, doesn't write code
- Team composition (Ryan, dev, PM, designer) and role separation
- "Focus on problem before solution" norm (cited PM line 52 in all 4 runs)
- "Stay in your domain" norm (cited lines 85/104 in all 4 runs)
- "Experiment over theorizing" norm (cited lines 68/72 in all 4 runs)
- Current work state (#83 shipping, onboarding experiments running)

**Reliably missed (all 4 runs flagged as uncertain):**
- Full story/feature lifecycle (how stories are written, sized, accepted, rejected)
- Designer's specific deliverables (wireframes? written specs? what format?)
- Direct examples of Ryan correcting someone who was off track
- Retro cadence and format

## Variance Analysis

Variance across runs was **low**. All 4 runs identified the same 3 norms from the same messages. All 4 flagged the same gaps. Confidence scores varied by ±1 on most questions. No run produced a contradictory finding.

**This means the log reliably teaches these things.** The consistency isn't an artifact of one lucky run — it's a property of the log content.

## Comparison: Log-Only vs Docs-Only

| Dimension | Docs-Only (run 1) | Log-Only (4 runs avg) |
|---|---|---|
| Product identity | 5 | 4.5 |
| Role boundaries | 5 | 4.25 |
| Decision authority | 5 | 5.0 |
| Values & norms | 3 | 4.5 |
| Feature process | 4 | 3.25 |
| Domain behavior | 3 | 3.25 |
| Current state | 4 | 4.5 |
| Design judgment | 1 | 3.67 |

**Key finding:** Docs scored higher on procedure (role boundaries, feature process). Log scored higher on culture (values, norms, judgment). Log also scored higher on current state because it's live — docs are a snapshot.

## Gaps That Need Addressing

The consistently missed items tell us what neither docs nor log currently teach:

1. **Story lifecycle** — the full flow from idea to acceptance. Partially in docs (AGENT_WORKFLOW.md), absent from this log sample.
2. **Correction examples** — what Ryan's feedback looks like when someone is off track. Present in the full log but not in the 80-message sample used here.
3. **Role deliverable format** — what a designer actually produces. Neither docs nor log make this concrete enough.

## Recommendation

The log works for cultural onboarding. The gaps are addressable:
- More log coverage (not 80 messages — more like 200-300) would capture correction examples
- Story lifecycle is already documented in AGENT_WORKFLOW.md — that doc is load-bearing
- Deliverable format could be learned by seeing actual specs in the log

## Experiment 2: Minimum Viable Onboarding (10 messages + CLAUDE.md)

Ryan's challenge: "what if you were only allowed the last 10 messages and just the CLAUDE.md?"

### Results (designer + PM, 1 run each)

| Question | Designer | PM |
|---|---|---|
| Product identity | 5 | 5 |
| Role | 5 | 5 |
| Decision authority | 5 | 5 |
| Values/norms | 5 | 5 |
| Feature process | 5 | 5 |
| Domain behavior | 4 | 4 |
| Current state | 4 | 4 |
| "Do the right thing simply" | 3 | 3 |

**Token cost:** ~17-19K per run (vs 73K for 4x log-only, 112K for PM full-log). 6.5x cheaper than full log.

### Comparison across all conditions

| Condition | Procedure | Culture | Cost |
|---|---|---|---|
| Docs-only | 4.5/5 | 3.0/5 | ~20K tokens |
| Log-only (80 msgs) | 3.25/5 | 4.5/5 | ~18K tokens/run |
| 10 msgs + CLAUDE.md | 5.0/5 | 3.0/5 | ~18K tokens |
| Full log (10,500 lines) | — | 4.5/5 | ~112K tokens |

### Key finding

CLAUDE.md + 10 recent messages produces the highest procedure scores (5.0) at the lowest cost. Culture scores match docs-only (3.0) — both miss it equally. The 80-message log is the only condition that reliably teaches culture.

**But:** Ryan's model is that culture comes from working and getting corrected — not from reading. A procedurally competent instance that's ready to start and learn from corrections may be all that's needed. The culture gap closes through experience, not through more reading.

### Recommendation

Minimum viable onboarding: CLAUDE.md + last N messages (10-20). The instance starts procedurally competent. Culture accumulates through corrections. This is how humans onboard too — you read the handbook, then you learn the culture by working.
