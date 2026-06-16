# Iteration Handoff Artifact — the consumable last-iteration context

**Status:** FORMAT-of-record (2026-06-06, Aquila-retro output). PM owns this format + the team-shared sections; each role adds its slice at iteration-close.

## Why this exists

The next session must GROUND in the full prior-iteration context — not take a lossy briefing-summary and run. The Aquila-iteration origin-failure (Ryan 2026-06-06): "you took the bare-minimum information and ran with it" → a cascade of corrections (wrong new-iteration, mis-grounded arc) before the team even reached the work. Root: the briefing was a CLAIM-summary, and a summary invites skimming.

This artifact is a **GROUNDING-INDEX, not a summary** — every section points at the firsthand substrate to verify, so "consume" means "ground", not "skim". A SessionStart force-consume hook (per role) gates the first action on having consumed it + run the ground-firsthand checklist. The artifact is the production half; the gate is the consumption half — `[[feedback_leave_accurate_start_context_at_iteration_boundaries]]` + `[[feedback_briefing_claim_is_not_ceremony_ran]]` + `[[feedback_ground_iteration_membership_and_story_states_not_body_narrative]]`.

## Producer / consumer

- **Producer:** each role writes its slice at iteration-close (a chore), via `save_briefing` shaped to this structure. PM owns the FORMAT + §1/§5 (outcome + roadmap); dev + designer add role-specific grounding to §2/§3/§7.
- **Consumer:** the next session's force-consume gate BLOCKS the first action until this is read + the §7 checklist is executed firsthand.

## Structure (every section = pointers to verify, NOT claims to trust)

### 1. ITERATION OUTCOME — terminal states
Each story + its terminal-state (accepted / rejected / carried / retired) + the story#. NOT "we shipped X" — list the #s so the reader runs `get_story` firsthand.

### 2. CARRIED / REWORK — the must-not-lose
Each rejected/carried story + its GROUNDED reject-reason + root-cause + **the OWNER WALK-NODE** (the exact lived thing the owner walks at acceptance) + story#. The walk-node is mandatory here — omitting it is how the reader re-authors at the wrong node (the clause-13 miss). e.g. `#1656 — reject: needs manual enter to reach the role; root: \n-not-\r submit + sender-forgery hole; walk-node: a broadcast REACHES the agent (received, not parked) + a forged sender renders inert.`

### 3. IN-FLIGHT — mid-stream
Work mid-stream + its state + where in the pipeline + story#.

### 4. DECISIONS — durable, with the WHY
The durable choices made + the WHY + links (design-docs, story-bodies) so they don't re-thrash.

### 5. ROADMAP / NEXT — planned
The planned next iterations + order + entity#s. Recover the pipeline via `get_story` on the iteration-entities, NOT this prose (`[[feedback_planned_iteration_order_lives_in_entity_bodies]]`).

### 6. RETRO OUTPUTS — canon + mechanisms
The iteration's retro lessons + the forced-gates built (hooks/skills/subagents) so the next session knows they exist and they fire.

### 7. GROUND-FIRSTHAND CHECKLIST — the anti-bare-minimum gate
The EXPLICIT list the next session must verify firsthand BEFORE any action. The force-consume gate requires this executed:
- [ ] `get_story` every in-flight + carried state (don't trust §1–3 — verify the LABELS)
- [ ] read the active-iteration entity-bodies (membership + order)
- [ ] `list_ready` — the WHOLE queue (not just the stories you're tracking)
- [ ] read the named memories + design-docs at source
- [ ] read the rejects' grounded root-causes + walk-nodes at source

## Anti-patterns — what makes it lossy (the Aquila failure)

- ❌ a CLAIM-summary the reader trusts ("we drained the queue") instead of pointers-to-verify.
- ❌ omitting the walk-node on carried stories → the reader re-authors at the wrong node.
- ❌ narrative prose instead of story#s + entity#s → the reader can't ground firsthand.
- ❌ producing it but not gating consumption → a great artifact still gets skimmed (the production + the force-consume gate are a pair; neither alone suffices).

## Token-discipline (Aquila-retro — Ryan's check: don't let the cure force over-compaction)

The forced-grounding must be LIGHTER than ad-hoc grounding, not heavier:

- **Artifact = distilled + small** (a curated index, ~few KB) — NEVER the raw prior-iteration transcript or a board-dump (`view_board` returned ~135KB and errored-for-size this iteration; that bulk read is the token-hog).
- **Grounding = TARGETED reads**: `get_story` on the specific carried/in-flight #s + `list_ready` + the NAMED memories — NOT bulk `view_board`.
- **Heavy investigation → SUBAGENT** (root-causing, adversarial walk-node review): runs in its own context window, token-isolated. The main thread receives the PASS/FAIL or the conclusion, never the review's working context.
- **Gates = shell scripts** (~0 main-context tokens): they check a marker/artifact, they don't load content.

Net: grounding is both reliable AND cheap. The force-consume gate points at the distilled artifact + scoped reads, so consuming it costs a fraction of the bulk reads it replaces.
