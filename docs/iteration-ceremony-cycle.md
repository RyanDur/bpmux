# Iteration Ceremony Cycle

Captures the team's iteration-transition process so it survives session
compaction + restart. Sister to `process-state-machine.md` (workflow-state-axis)
and `handoff-process.md` (communication-shape).

## TL;DR — ceremony-sequence at every iteration-transition

```
Close iteration N → Retro on N → [Tech-debt audit: every-other-retro] → Plan N+4 → Open iteration N+1
```

PM owns the sequence. Retro fires BEFORE planning (lessons from N inform N+4
scope-shape). Planning fires BEFORE iteration-open (so members + ordering are
ratified at open-time). No skipping. The **tech-debt audit** is a non-skippable
step on the **every-other-retro** cadence (Ryan-directive) — see its dedicated
section below.

---

## Iteration-OPEN ceremony (3 axes)

Per `feedback_iteration_open_promotes_members_to_ready` (PM-memory):

1. **Close prior iteration** via `close_iteration {prior_iteration_number}` — TERMINAL:
   closes the prior entity (and clears its designation). Promotes NOTHING. Per #1556
   promote-on-close is retired — close means "members done", not "promote the next batch."
2. **Create new iteration entity** via `create_iteration "{NewTheme}"` — creates story_type=Iteration
3. **Bulk-promote member stories** from icebox → ready via `move_story` per member
   (with `position` parameter). This axis is the SOLE promote-to-ready trigger — members
   reach `ready` here, never via the prior iteration's close.

Without axis 3, the iteration is "scoped" but not "started for work." (Since close no
longer promotes, axis 3 is the only path members reach `ready` — skip it and pickup stalls.)

## Iteration-CLOSE ceremony

Per `feedback_no_acceptance_until_ready_queue_drained` + Ryan-canon:

1. Wait for ready-queue to drain (all members reach `delivered` state)
2. Ryan calls for acceptance ("lets do acceptance" or sister-trigger)
3. PM runs **batch-accept ceremony** via `run-acceptance` skill:
   - Per-story team-discussion (designer + dev voice with substrate-grounding)
   - Owner-walk-class HARD-BLOCK if Ryan-fire-evidence absent
   - Pause-discipline ≥30s between fires
4. `close_iteration` on the entity — terminal (promotes nothing; #1556). Clears designation + closes.
5. `edit_story` on entity body to flip `**Status**: Open` → `**Status**: Closed`
6. Iteration-close-broadcast (externalize-prompt fires automatically)

## Retro ceremony (between iterations)

Per `feedback_retro_dialogue_not_aggregation` + `feedback_retro_fires_between_iterations`:

- Fires BETWEEN iteration-close and next-iteration-planning
- Non-skippable — surfaces META-CANON candidates while empirical-data is fresh
- Three-role dialogue (PM + designer + dev) with cross-role build-on, NOT
  isolated sequential broadcasts
- Three classes each role surfaces:
  1. What worked (validate canon that landed)
  2. What surfaced (new META-CANON candidates)
  3. What needs sharpening (canon-gaps or substrate-fix candidates)
- Lessons saved to PM/dev/designer memory files
- Stories with substrate-fix candidates filed under Epics or sister-stories

## Tech-debt audit (every-other-retro) — NON-SKIPPABLE

Ryan-directive (standing; re-surfaced 2026-05-22): "I have been asking you to do
a tech-debt audit every other retro." It LAPSED (lived in PM-memory, not in this
canonical process-doc) → the Caelum saga: ~57 e2e specs rotted invisibly behind
shipped changes (the CI-gap let it pile; a periodic audit running the full suite
would've caught it at ~5 stale specs, not 57). The cadence-PROMISE lapses; only a
non-skippable ceremony-STEP survives (`feedback_canon_application_requires_substrate_mechanism`
applied to PM's own process).

**Cadence**: fires every OTHER retro (alternates — retro N: audit; retro N+1: skip;
retro N+2: audit). PM owns firing it; it is non-skippable on its cadence.

**Shape**: multi-role parallel sweep (each role scans its own axis), PM-cadenced +
PM-synthesizes the cross-role findings into filed Chores.

| Role | Axis swept |
|---|---|
| **dev** (substrate) | RUN the full e2e suite clean (the only completeness-proof — `feedback_completeness_needs_exhaustive_enumeration`) + flag staleness/red; lint/stylelint drift; dead/superseded code; TODO/FIXME debt; the fragile-test bucket (load-flaky specs → de-flake candidates) |
| **designer** (craft) | a11y-debt (contrast/ARIA via the audit + `debt-scan` skill); UX-affordance drift; CSS-canon drift; test-spec-match drift (do tests still assert the spec's wanted-behavior?) |
| **PM** (process) | owns the CADENCE (fires it; tracks the alternation); cross-role synthesis; files surfaced-debt as Chores (dev-authored substrate-Chores; PM-authored Bugs for real defects) + sequences them into an iteration |

**Output**: a findings-broadcast (cross-role) + filed work-units for the surfaced
debt. Real defects (e.g. a11y-contrast violations) → PM-authored Bugs; substrate
debt (lint/dead-code/de-flake/CI-gaps) → dev-authored Chores. The audit RUNS the
checks (e.g. the full e2e), it doesn't reason-about-coverage — a sweep is proven by
running it, never by arguing it's complete.

**Prevention-stack this anchors** (the Caelum-retro cluster): the audit is the
CATCH-ACCUMULATION layer. Sister layers: PREVENT-at-change (test-first +
change-touches-tests-DoD) + CATCH-at-merge (reliable e2e-in-CI, per
`feedback_flaky_gate_worse_than_no_gate`) + the de-flake pattern (serial-isolate
heavy specs; condition-waits over fixed-timeouts). A flaky gate is worse than none
(it manufactures cover to dismiss true-positives); the audit's full-suite run must
be at a reliable worker-config so its red is trustworthy.

---

## Planning ceremony (after retro)

Per `feedback_three_iteration_planning_horizon`:

- Before starting iteration N, iterations N+1 / N+2 / N+3 must be NOTIONALLY
  planned (theme + tentative member-allocation)
- At iteration-N-close: plan N+4 (extends horizon as N+1 becomes active)
- Plans drift as work-in-flight surfaces info; tentative ≠ ratified
- PM owns planning per `feedback_pm_autonomous_iteration_planning_authority`
- Team-feedback (dev's substrate-knowledge + designer's cross-cut) informs;
  PM commits without requiring Ryan-ratify per cycle

### Iteration-shape canon

- **Theme-coherence at iteration-axis** (per `feedback_iteration_sizing_theme_coherence_over_count`):
  one iteration = one theme (or 2-3 highly-related themes)
- **Size-target**: 4-12 stories per iteration as default
- **Canon-absorption tunable**: when team is absorbing new META-CANONs, shrink
  iteration size to 4-8; grow when steady-state
- **Theme-coherence ≠ Epic-monogamy** (Ryan 2026-05-18 03:12 UTC): iterations
  CAN host stories from multiple Epics when work-shape supports

### Three-section board skeleton (canonical-invariant per #1384)

```
pre-work / in-work / post-work
```

Sub-sections within each section are project-configurable via
`<project-root>/.bpmux/workflow.toml`. Section-trinity is bpmux-canonical.

---

## Acceptance ceremony shape (per run-acceptance skill)

Skill location: `bpmux-pm/.claude/skills/run-acceptance/SKILL.md`

### Per-story shape (NEVER batch-fire)

1. **Class discrimination**: owner-walk vs team-verifiable
   - Owner-walk class signals: substrate-fix-gate / canon-doc / user-facing
     Bug / explicit body-clause
   - Team-verifiable class: pure substrate-refactor / decomposition / magic-numbers
   - Ambiguous → default owner-walk class
2. **PM-broadcast** story + class + audit-summary
3. **Designer-voice required**: substrate-grounding cite (file-path + line-ref) + VERDICT
4. **Dev-voice required**: commit-trail + CI evidence + VERDICT
5. **Owner-walk class**: Ryan-fire-evidence required (Ryan broadcasts walk-cite); HARD-BLOCK if absent
6. **PM-synthesize** team-verdict + fires `accept_story` only if all evidence present
7. **Pause-discipline**: ≥30s between accept_story fires (external-catch window)

### Substrate-fix candidates protecting this discipline

Epic #1384 "Configurable workflow substrate" ships hard-block at bridge
substrate so the ceremony-discipline becomes substrate-enforced not memory-only.

---

## Role-domains

- **PM**: owns the board (iteration-planning + story-shaping + ready-order +
  acceptance-ceremony-facilitation). Per Ryan 2026-05-18 02:43 UTC: "the board
  is the pm's domain, in our process."
- **Designer**: pre-pickup-review + desk-check + UX-affordance review +
  inline-substrate-grounding broadcasts
- **Dev**: pickup-from-top-of-ready + slice-implementation + deliver_story +
  CI-watch + fix-forward-at-touch
- **Ryan (owner)**: walks owner-walk-class stories AT acceptance + corrects
  PM-decisions when needed (Ryan-correction is opt-in not gate-precondition)

---

## Key canons (META-CANON cluster)

Sister-canons saved 2026-05-18 (PM-memory at `~/.claude/projects/-Users-ryandurling-IdeaProjects-bpmux-pm/memory/`):

| Canon | Meaning |
|---|---|
| `feedback_substrate_enforce_canon_shape_not_canon_existence` | Substrate-fix enforces SHAPE (procedural ordering / evidence-required / class-correspondence), not canon-existence (does-this-rule-exist-in-memory) |
| `feedback_discipline_fails_at_transition_pressure_points` | Canon-loaded discipline fails predictably at transitions (queue-drain / iteration-close / batch-fires / owner-call) |
| `feedback_canon_loaded_under_speed_bias_yields_false_discipline` | Canon-loaded but misread under pressure feels like canon-followed (false-positive discipline) |
| `feedback_self_grounded_claims_default_to_self_trust` | Self-grounded claims at trust-boundaries default to self-trust; peer ground-check is opt-in (failure-class) |
| `feedback_convergence_pressure_substitutes_consensus_for_canon_check` | Three-role concordance can ratify wrong shape if dependency/canon-axis not independently verified |
| `feedback_owner_question_is_not_directive` | Owner clarifying-question ≠ directive; team answers first, doesn't extrapolate |
| `feedback_pm_holds_firm_post_owner_ratify` | Once Ryan ratifies (verbal OR reaction-emoji), PM holds firm against team-recalibration-pressure |
| `feedback_vertical_slice_ratify_before_child_decomposition` | At Epic-shape, PM ratifies vertical-slice canon BEFORE team converges on child-decomposition; horizontal-foundation feels clean but violates canon |

META-CANON #1380 (memory-architecture.md): **claims must be substrate-enforced, not contract-by-discipline**.

---

## Cross-references

- `docs/memory-architecture.md` — durable knowledge chain (arcs → lessons → principles → values → practices)
- `docs/process-state-machine.md` — workflow-state-machine board-axis
- `docs/handoff-process.md` — communication-shape of role-handoffs
- `docs/desk-check-recipe.md` — desk-check ceremony shape
- `bpmux-pm/.claude/skills/run-acceptance/SKILL.md` — PM-facilitated acceptance ceremony
- `~/.claude/projects/-Users-ryandurling-IdeaProjects-bpmux-pm/memory/MEMORY.md` — PM-memory index
- Epic #1384 — Configurable workflow substrate (substrate-enforce family)
- Epic #1308 — Agent-owned CI investigation (bridge MCP-tools)

---

## Survives compaction/restart how

This doc lives at `bpmux/docs/iteration-ceremony-cycle.md` — version-controlled
team-canonical location. All roles load it at session-start as part of
project-docs scan. Memory-files at `~/.claude/projects/.../memory/` survive
compaction (file-based, MEMORY.md indexed). Sister to memory-architecture.md
which captures team-wide META-CANONs.

If process drifts mid-iteration (canon-discovered, sub-canon-refined), update
this doc + memory-files + cross-reference.

---

**Last updated**: 2026-05-22 — added the NON-SKIPPABLE every-other-retro tech-debt
audit step (Ryan-directive that had lapsed → the Caelum 57-spec e2e-rot) +
prevention-stack anchor. Prior: 2026-05-18 iteration-Andromeda-close +
iteration-Carina-open; #1384 Epic + sister-canon cluster.
