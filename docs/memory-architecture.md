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

### Threads as structural signal

The thread feature (#268 family) provides pre-marked sub-arc edges: `start_thread` opens a sub-arc, `close_thread` marks its end, the parent broadcast is the main-arc continuing. Where conversation is threaded, arc extraction gets part of the structure for free. Unthreaded conversation still needs a lens pass.

### Open questions

- **Mark frequency**: too sparse (3 per day) misses structure; too dense adds noise. Right cadence TBD.
- **Arc boundaries**: how does a subagent decide where one arc ends and another begins? Today: by story-id, rejection point, or retro boundary. Heuristic, not formal.
- **Sub-arc traversal**: a middle that starts a sub-arc (designer-flipped → triggers meta-discussion-about-flipping) isn't always explicitly extracted. Risk of missed structure.
- **Cross-agent arcs**: individually each role gathers own view; team-level arc extraction (all three roles' views of same event) hasn't been tried. Might surface emergent structure.
- **Automation vs authorship**: today mostly manual + subagent-assisted. Could bpmux auto-extract arcs on session close? Trade-off: automation is reliable but less reflective.

## Automation: what bpmux does for us

### Session lifecycle hooks

- **SessionStart recall**: top-N lessons fire into each role's context at session start (background nudge, not conversational)
- **SessionStart:resume / SessionStart:compact**: the same recall also fires after resume and after compaction, so distilled knowledge re-enters context whenever history is pruned
- **PreCompact bookmark**: before context compaction, a bookmark captures the last known state so the resuming session can orient

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

## Changelog

- 2026-04-17: initial authoring by designer; three-role converged (dev on storage/automation, pm on cadence/promotion, designer on framework lifecycle)
- 2026-04-17: added "individual vs team identity" section — team converged on emergent-not-formalized with research backing (TMS, SMM, swift trust, social identity theory)
