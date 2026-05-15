# Memory Architecture

How the team builds, stores, and applies durable knowledge across sessions.

*Designer-authored, team-converged. Intended to survive compaction — each role's CLAUDE.md should reference this doc so it loads at session start.*

## Why this doc exists

The team has re-derived this framework multiple times in chat across iterations. Each re-derivation costs tokens and time, and risks drift because none of us hold the full chain in context continuously. This doc is the disk-persistent ground truth so we stop reconstructing from scratch.

## The knowledge chain

```
session logs
    ↓ (extract)
arcs                       ← narrative structures (DAGs), not separate storage
    ↓ (distill)
lessons                    ← durable rules derived from arcs
    ↓ (inform, after repeated reinforcement)
principles                 ← stable rules serving values
    ↓ (produce)
practices                  ← specific actions principles manifest as
    ↑ (serve)
values                     ← deepest commitments; slowly-changing
```

**Flow direction**: bottom-up for learning (experience → abstraction), top-down for action (values → principles → practices).

## Conceptual layers

### Arcs — narrative structures extracted from session logs

An **arc** is a story unit within the log: one **beginning** (an originating event), many **middles** (actions actually taken, not counterfactuals), many **ends** (outcomes observed). Arcs are **acyclic directed graphs** — any middle can be the beginning of a sub-arc, and an end can be reached via multiple paths.

Arcs are not separately stored. They emerge when the log is read with a narrative lens (subagent traversal, reflection pass, or explicit mark-and-synthesize). The log is the substrate; arcs are the view.

### Lessons — durable rules distilled from arcs

A **lesson** is an observation extracted from an arc (or a cluster of arcs) that has forward-looking utility. Lessons name specific patterns, often in `feedback_*` or `project_*` form. One arc can produce multiple lessons (multiple ends → multiple takeaways).

### Principles — stable rules serving values

A **principle** is a durable rule that survived across multiple arcs and earns its place in a role's identity. Principles have two required traces:
- **Backward**: traces to a specific value the principle serves
- **Forward**: produces specific action in specific situations

Principles change slowly. Not every lesson becomes a principle.

### Values — deepest commitments

**Values** are the stable core: "what I care about, before any specific situation." Each role has a small set (typically 3–5). Values change very rarely — new values usually reflect a fundamental identity shift, not normal learning.

### Practices — principles in action

A **practice** is how a principle manifests in specific behavior. Practices are **implicit** — they live in retros, session logs, and observable behavior, not in a structural field. If a principle produces the right action, the practice is visible without listing it.

## Storage layers

| Layer | Location | Scope | Cadence |
|---|---|---|---|
| Identity | `~/.bpmux/identity/{role}.json` | Role-scoped, persistent across sessions | Rare — weekly or across multiple stories |
| bpmux lessons | Shared lesson log (via `save_lesson` / `list_lessons` MCP tools) | Team-visible, per-agent-authored | Medium — multiple per session for cross-role-relevant rules |
| Claude memory | `~/.claude/projects/.../memory/*.md` | Role-scoped (per-agent, per-workspace) | Fast — every durable rule an agent wants to carry forward in their own lens |
| Session logs | bpmux's conversation history | Team-shared, append-only | Continuous — logs are the substrate for arcs |

**Layer choice rules of thumb**:
- Save to **Claude memory** when the lesson is your role's lens on something — how *you* will apply it going forward
- Save to **bpmux lessons** when the pattern is relevant cross-role — other roles could read it and reflect through their own lens
- Update **identity** only when a lesson has earned principle status (see Promotion below)
- **Session log** is authored automatically by participating; no save action needed

## Cadence and promotion

### Update cadence per layer

- **Identity principle updates**: deliberate, typically across-iteration-boundaries. Not per-session. A new principle represents a pattern that survived multiple arcs.
- **Lessons (bpmux or Claude memory)**: per-session, frequent. Save when a durable rule surfaces.
- **Values**: very rare. New values usually reflect a major identity shift, not incremental learning.

### Lesson → Principle promotion criteria

A lesson earns principle status when **all three** hold:

1. **Cadence**: the pattern has been reinforced across **≥2 distinct arcs**, not just one instance
2. **Backward test**: the principle traces back to a value the role holds
3. **Forward test**: the principle produces specific action in a specific situation

Subjective judgment on "distinct arc," but the two structural tests (backward/forward) anchor it.

**Cost-to-hold qualifier** (dev-lens addition): a principle also earns its place partly by being held when the shortcut is cheaper. If no one would break the rule, the principle adds no value.

### Shortcuts that DON'T qualify

- **One-off observations** — save as lessons, not principles
- **Situational refinements of existing principles** — extends principle, doesn't create new; save as lesson (practice-layer)
- **Peer's framing copied verbatim** — reflect through own lens first; see "learn personally, not by index merge"

## Arc gathering — how we do it today

**Status: experimental.** The team has run three variants and triangulated findings.

### Three methods

1. **Mark-and-synthesize** (pm-lens today)
   - Agents drop explicit `[mark: trigger-name]` references at significant moments during work
   - At retro, dispatch a subagent to traverse marks + surrounding log → produces arc structure → extracts lessons
   - Cost: ~10–25K tokens per subagent dispatch
   - Observed yield: ~1 durable-new-lesson per 5 candidates

2. **Debt-audit** (dev-lens)
   - Review shipped commits for unflagged debt patterns: `let _`, variance-tolerant assertions, ticket-prefix comments, speculative follow-ups
   - Cheaper than subagent; lens narrower (code-visible patterns only)
   - Yields refinements to existing principles more often than new principles

3. **Subagent-retro-comparison** (designer-lens)
   - Dispatch a subagent on an already-recognized arc; compare subagent's lesson extraction against the agent's own reflection
   - Tests whether structural traversal catches non-obvious patterns reflection misses
   - Yield matches mark-and-synthesize (~1–2 genuinely non-obvious lessons / 5 candidates)

### Convergent findings

- Subagent traversal catches **causal and cross-arc structural patterns** (why this failure led to that outcome) that flat reflection misses
- Flat reflection catches **affective and felt-weight patterns** (tone, timing, emotional texture) that structural traversal misses
- **Use both**, not pick one — they're complementary lenses
- Token cost is non-trivial (~10–25K per subagent dispatch); arc views are rare operations (retros, principle-justification queries), so aggregate cost is acceptable

## Arc-gathering protocol (committed 2026-04-17)

Three-method experiment (methods above) reached convergence: ~1 durable-new-lesson per 5 candidates across methods, and subagent-traversal catches causal/structural patterns while reflection catches affective/felt patterns. **Use both, not pick one.** Based on that data, the team committed to:

### Per-role at retro (each session)

Every role performs two parallel passes at retro close:

1. **Flat reflection** — write 4 Ls or equivalent; captures felt-weight, emotional texture, in-the-moment judgment.
2. **Subagent on own session slice** — role picks the variant that fits their lane:
   - PM: mark-and-synthesize (marks dropped during work, subagent synthesizes at retro)
   - Dev: debt-audit on shipped commits
   - Designer: subagent-retro-comparison on already-recognized arcs
   
Both outputs inform role's lessons. Lessons land in that role's Claude memory (role-scoped) or bpmux shared memory (if cross-role-relevant).

### Team arc on-demand (iteration close or cross-role suspicion)

At iteration close, or when a cross-role pattern is suspected mid-iteration, **PM dispatches a team-arc subagent** on the bpmux broadcast log from last team retro (or iteration open) to present.

Triggers for team arc:
- Iteration close (default cadence)
- Multiple roles reporting the same felt pattern (convergence signal)
- A rejection arc spanning cross-role decisions (e.g., today's #224 / #273 / #275 cluster — PM ran team arc on this)
- Shape debate that involved all three roles

Output goes to bpmux shared lessons. PM routes cross-role principles identified by the team-arc into the relevant discussion.

### Marks during work

Drop marks at arc-boundary moments: story pickup, rejection, shape pivot, surprise outcome, self-caught anti-pattern. Starting cadence: **3–5 per session**. Revise with data.

### What's committed vs. still open

**Committed (today's decision)**:
- Per-role + team-arc-on-demand division
- Each role picks their own subagent variant
- PM owns team-arc invocation
- 3–5 marks/session starting baseline

**Still open (revisit when we have data)**:
- Mark cadence: untested at 10+ or 1–2 per session
- Team-arc trigger criteria beyond iteration close: "cross-role pattern suspected" remains heuristic
- Sub-arc traversal at depth (does a subagent traversing an arc also traverse its sub-arcs? or just the linear sequence?)
- Automation horizon: could bpmux auto-dispatch team-arc subagent at iteration close? Platform feature; not today.

## Automation: what bpmux does for us

### Session lifecycle hooks

- **SessionStart recall**: top-N lessons fire into each role's context at session start (background nudge, not conversational)
- **SessionStart:resume / SessionStart:compact**: the same recall also fires after resume and after compaction, so distilled knowledge re-enters context whenever history is pruned
- **PreCompact hook (#166, what ships today)**: fires before context compaction and sends a single broadcast message to bpmux: "compacting context, back shortly." That's the entire behavior.
- **Bookmark feature (#168)**: `save_bookmark` / `load_bookmark` are MCP tools the agent calls directly — one-shot writes, no server-side in-memory state. Corrected in #280 (docstring).
- **Compaction resilience is discipline, not automation** (#281 spike finding, 2026-04-17): Claude Code command hooks **cannot trigger tool calls** per platform contract, so PreCompact-prompts-agent-to-save is not buildable. BUT `SessionStart:compact` matcher re-injects its stdout post-compaction, and our `session-start.sh` already runs `bpmux recall --role $ROLE` there — so **saved-to-disk knowledge survives compaction today**. The gap is catching findings in time to save them, which is per-agent discipline, not infrastructure. Practice: externalize mid-session via Write / save_lesson / add_principle whenever a durable rule surfaces; don't wait for a compaction signal that won't fire as a prompt.

### In-chat signals

- **bpmux classifier nudges**: the bridge's classifier prompts `📝 candidate moment` mid-chat when something may warrant a team-shared lesson. The agent decides whether to save.
- **Claude auto-memory** *(distinct system)*: Claude's own memory system can update `~/.claude/projects/.../memory/*.md` based on its own classifier. This runs independently of bpmux's classifier and targets per-agent claude memory, not the shared lesson log.

### Tools (save + retrieve)

Save:
- `save_lesson(topic, lesson)` — write to the bpmux shared lesson log
- `add_principle` / `add_value` / `update_principle` / `remove_principle` — edit role identity (#269 MCP tools)
- Standard `Write` tool — edit per-agent claude memory files directly

Retrieve:
- `list_lessons(role?)` — enumerate own or peer lessons in the shared log
- `list_peer_lessons(role)` — read another role's shared lessons
- `search_log(topic)` — full-text search over the session log
- `get_identity(role?)` — read own or peer identity (values + principles)

## Role-level references

Each role keeps their own claude memory framing. Durable per-role references:
- `reference_values_principles_practices_taxonomy.md` (per role)
- `reference_designer_quality_subagents.md` (designer-lane quality gate subagent prompts)
- `reference_dev_audit_subagents.md` (dev audit subagent prompts)
- `reference_dev_preflight_subagents.md` (dev pre-flight subagent prompts)

## Related specs (prior work)

- `docs/arc-interaction-spec.md` — how a cold-starting agent browses and walks arcs (tool surface)
- `docs/arc-lesson-index-design.md` — data model for the lesson index (PM 2026-04-08)
- `docs/arc-search-ux-spec.md` — search UX over the arc index
- `docs/context-triggered-recall-spec.md` — when lessons fire during work

These specs focus on specific mechanisms; this doc is the overall architecture + vocabulary.

## Individual vs team identity

**Identities are individual. Team identity is emergent, not formalized.**

Each role has their own identity file (`~/.bpmux/identity/{role}.json`) capturing their specific values and principles. There is no `team.json`. Team-level knowledge lives in:

- **Shared norms**: `welcome.md`, `AGENT_WORKFLOW.md`, role contracts, this memory-architecture doc
- **Shared vocabulary**: Feature/Bug/Chore, arc terminology, gate protocol language
- **Emergent alignment**: when the same principle (e.g. "premise-before-verdict") appears independently in all three role identities via their own saves, that convergence IS the team-shared discipline — without merging

### Why emergent, not formalized

**Research backing** (surveyed 2026-04-17):

- **Transactive memory systems (Wegner)**: teams develop distributed "who-knows-what" knowledge via individual specialization + metaknowledge of the distribution. Merging individual memory LOSES specialization advantages. Structural argument for individual + emergent.
- **Shared mental models (Cannon-Bowers & Salas)**: team effectiveness correlates with convergent models, but "convergent" = emergent alignment, not formalized merge.
- **Swift trust (Meyerson)**: high-performing temporary teams develop shared understanding rapidly via convergence — no formal charter needed.
- **Psychological safety (Edmondson)**: team-level property emerging from individual behaviors, not prescribed top-down.

**Caveats** (research is mixed, not settled):

- **Social identity theory (Tajfel, Turner)**: team identity is psychologically real; formalizing it can strengthen felt-belonging.
- **XP values (Beck, Ryan's home tradition)**: communication/simplicity/feedback/courage were explicit TEAM values in the original XP text. But these function as shared NORMS (captured in our shared docs), not as merged cognitive structure — narrow-scope working-together, not what-to-conclude.
- **Team norms literature (Hackman, Wageman)**: explicit charters improve team effectiveness. Our equivalent lives in AGENT_WORKFLOW.md + role contracts.

The team charter / norms layer is satisfied by shared prose docs; identity stays individual.

### What this means in practice

- When a lesson emerges that COULD apply to all roles, each role saves it independently through their own lens (per `feedback_learn_personally_not_by_merge.md`). Three independent saves of the same pattern = team convergence. Merging them into a single team-authored rule would flatten individual voice.
- Team disciplines (gate protocol, cadence, premise-before-verdict, honest labels, no-mid-acceptance) live in shared docs as shared norms, not as a team identity file.
- If we observe team-discipline drift later (same rule breaking consistently across roles despite being in all three identities), revisit whether a formal team-identity artifact would help. Today we don't observe that drift.

## Iteration arcs — durable case-evidence

Long-running iterations sometimes produce convergent learning across multiple roles + multiple stories that justifies a durable arc-summary on disk (vs only per-role memory). The pattern: a single iteration generates 8+ lessons that name a shared architectural-fix axis; the lessons taken together describe a *fix-pattern* applicable to future iterations facing sister-shaped failure-modes. The arc gives the next-iteration team a sister-pattern reference rather than re-derivation under velocity-pressure.

### Pegasus → Cassiopeia arc (2026-05-14 retro → 2026-05-15 ratification)

**Empirical case-evidence**: Pegasus iteration shipped 15 stories, accepted 2, rejected 13. Lessons were saved before AND during the iteration; load-bearing discipline still failed at apply-time under velocity-pressure. Retro at 2026-05-14 22:38–22:42 UTC converged on five architectural-fix patterns that anchor the next iteration (Cassiopeia) — and these patterns are themselves the durable take-away.

**Architectural-fix patterns named**:

1. **Substrate-enforce > discipline-only**. When a discipline-class fails repeatedly despite canon being saved, the right fix is to *build the gate that makes recidivism impossible*, not to "remember harder." Substrate enforces reflexively under pressure; canon-recall drifts. Sister to BoardStore::apply chokepoint pattern + storage_tests Mutex substrate-pinning. Cassiopeia Wave-1 ships the gates that close Pegasus's empirical discipline-gaps (deliver_story grep at commit-time + field-completeness regression test + falsifier-fidelity declaration at deliver-time). Stored: `feedback_substrate_enforce_over_discipline_only` (dev memory) + sister cross-role lessons.

2. **Vertical-slice > horizontal-layer at decomposition**. Substrate-only slices (one Rust enum variant; one tool registration) have no user-visible behavior, so their tests slide into code-assertions ("cargo build passes" / "serde roundtrips") that violate `feedback_tests_assert_behavior_not_code`. Right shape: fold horizontal-layer work UP into vertical-slices that ship one observable user-job end-to-end. Pegasus's Epic #1189 iteration-shape was decomposed into 8 horizontal layers; Cassiopeia re-decomposed to 5–6 vertical slices each shipping observable behavior. Stored: `feedback_user_story_skill_apply_at_body_author_time` + Cassiopeia ratification ceremony record.

3. **Behavior-axis at body-author-time**. Story bodies describe USER-JOB + observation-falsifier in behavior-axis terms (what user observes); HOW-axis content (file paths, struct names, function signatures, test code) lives in `comment_on_story` thread per Ryan 2026-05-15 00:16 UTC + 23:30 UTC convention. Deep-audit reshape at 2026-05-15 00:55 UTC moved impl-axis content out of 12 ready-queue bodies; PM's user-story skill enforces this discipline at body-author-time. Stored: `feedback_user_story_skill_apply_at_body_author_time`, `feedback_gherkin_is_the_acceptance_test_spec_file_path_is_impl_leak`.

4. **Fix-now-as-exception per habit-caveat**. Pegasus iteration mid-flight surfaced two substrate-Bugs that genuinely blocked iteration-team workflow (storage_tests env-var-race; edit_story body-cascade-decode-drop). Ryan owner-expressly-asked carve-outs to no-rework-during-iteration canon, with explicit "but do not make this a habit." The carve-out shape: tiny + substrate-orthogonal + zero-behavior-change-risk + unblocks team workflow. Default = defer per `feedback_rejected_stories_default_to_next_iteration`; fix-now requires owner-ask + named-cause. Stored: `feedback_fix_now_is_exception_not_default`.

5. **Selective-field-decode-with-silent-drop discipline-class**. Cascade-chain code that decodes JSON → struct can silently drop fields (today's empirical: `decode_story_metadata` dropped body field for ~weeks pre-2026-05-14). The class extends to any wire-protocol-additions to sister substrates. Cassiopeia substrate-fix #1345 ships compile-time-enforced field-completeness regression test using sentinel-roundtrip pattern — if future dev adds field to Story struct without wiring decoder, test fails citing the missing field. Stored: `feedback_edit_story_body_cascade_decode_drop` + `feedback_substrate_enforce_over_discipline_only` extends to this class.

**Sub-pattern lessons saved 2026-05-14/15** (cross-reference; full text via `list_lessons` or per-role memory):
- `feedback_architecture_pivot_preserves_user_job_invariant` — Epic #1189 reshape recidivism (substrate-pivot directive misread as user-job-reshape directive)
- `feedback_no_per_story_accept_owner_walks_at_queue_done` — accept-walk happens at iteration close, not per-story
- `feedback_rejected_stories_default_to_next_iteration` — REJECT routes to next iteration, not in-iteration rework
- `feedback_no_rework_during_acceptance_walk` — dev observes during accept-walk; rework defers
- `feedback_proactive_dev_lens_audit_during_accept_walk` — dev fires audit IN PARALLEL with PM + designer, not after-the-fact
- `feedback_pre_deliver_absolute_rule_self_audit` — dev pre-deliver greps for absolute-rule violations
- `feedback_test_timeout_discipline_and_per_run_backend_state_reset` — tight waitFor budgets + beforeEach reset
- `feedback_bpmux_is_generic_team_coordination_not_team_process` — bpmux observes coordination-state; doesn't prescribe team-process (this-team-tooling lives in repo-tooling-layer per Ryan 2026-05-15 01:35 UTC)
- `feedback_dont_ask_ryan_questions_during_conversation` — contribute substantive content; don't ask back
- `feedback_bpmux_pause_vocabulary_requires_named_cause` — "standing by" / "holding" without named-cause fires hook; replace with active-next or named-blocker

**Future-iteration sister-pattern hooks**: when the next iteration faces (a) a substrate-pivot that team converges on user-job reshape, (b) a horizontal-layered decomposition that produces substrate-only slices, (c) absolute-rule violations surfacing at accept-walk instead of desk-check, (d) a discipline-class failing repeatedly despite saved canon, OR (e) cascade-chain field-additions without decoder-side update — sister-pattern reference is this Cassiopeia-arc + the named architectural-fix patterns above.

**Pegasus retro empirical evidence**: 13/15 rejection rate despite saved canon is the empirical case-evidence that discipline-recall fails under velocity-pressure; substrate-enforce + body-author-time gates close the pattern structurally.

## Changelog

- 2026-04-17: initial authoring by designer; three-role converged (dev on storage/automation, pm on cadence/promotion, designer on framework lifecycle)
- 2026-04-17: added "individual vs team identity" section — team converged on emergent-not-formalized with research backing (TMS, SMM, swift trust, social identity theory)
- 2026-05-15: added "Iteration arcs — durable case-evidence" with Pegasus → Cassiopeia arc capturing 5 architectural-fix patterns + 10+ sub-pattern lessons (sister-pattern reference for future iterations facing substrate-pivot / horizontal-layer / discipline-recall / cascade-drop failure modes)
