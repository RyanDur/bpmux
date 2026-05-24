# Explicit Handoff Process

Every pipeline transition is a directed message to the next person. No broadcasting into the void.

## Handoff Format

Every handoff names four things:
1. **Who** — the specific person ("PM", not "the team")
2. **What** — the artifact ("desk check #N", not "take a look")
3. **Action** — what they need to do ("review", "build", "accept")
4. **Urgency** — when ("now", "when ready", "after current work")

## Pipeline Transitions

> Per-role **full** responsibilities live in each role's **bpmux-identity** (`get_identity`) — durable, recalled at session-start. This doc holds only the cross-role EDGES (below) + the enforcement gap-map (bottom); it does NOT duplicate per-role responsibilities.

| Transition | Handoff |
|---|---|
| PM shapes + files a story | "Designer, #N filed — pre-pickup review?" → "Dev, #N in ready — restate before building." |
| Designer specs ahead | "PM, spec ready for #N at docs/[file]. Shape when ready." |
| Dev delivers (optimistic, post-push) | "Designer, delivered #N — desk-check (dual-gate: team-review + CI-green-on-head-sha-or-ancestry)." |
| Designer desk-check PASS | "Dev, #N passed — pull next (WIP-1)." + "PM, #N pooled for acceptance." |
| Designer desk-check FAIL | "Dev, #N failed — [reason]. Rework + redeliver (stays in-iteration)." |
| PM runs acceptance (queue-drained, Ryan-call) | **owner-class** → "Ryan, #N — walk + fire accept" (Ryan alone); **team-class** → PM fires on fresh dev+designer verdicts (team = the three agents PM+dev+designer; autonomous, no owner-wait); **chore** → dev fires. |
| Ryan accepts | `accept_story` closes. "Dev, #N accepted." |
| Ryan rejects | "#N rejected — [reason]. Carries to NEXT iteration (rework happens then)." |
| Iteration transition | PM: close N → retro on N → plan N+1 → open N+1 (non-skippable). |
| Queue empty | "PM, ready queue empty." |
| Blocker found | "Team, blocked on [what]. [Who] can unblock." |

## Rules

- Handoffs are directed messages (name the person; omit `to` only for true broadcast)
- The recipient acknowledges: "On it" or "After current work"
- If no acknowledgment in 60 seconds, the sender follows up
- Don't wait silently — if you're idle and nobody handed off to you, ask: "PM, what's next?"

## Role-boundary enforcement (gap-map: gated / skill / doc)

Honest enforcement-status of the edges above — where a boundary is substrate-GATED (can't drift), SKILL-enforced (invoked), or doc-only (recall-vulnerable → a substrate-enforce candidate). Roles VERIFY their own rows (src-cited, not recalled — a drift here defeats the purpose).

- PM owns board-MUTATIONS → **GATED** PM-only (dev-src-verified): move_story (#1548), close_iteration, add_story_to_iteration, label_story, set/unset_parent_epic, move-with-position.
- create_story → **GATED, typed carveout** (dev-src-verified, `tools_stories/creation.rs:25`): PM creates any type; **dev creates [Chore]-class only** (enforced via story-type check — gate, not mere convention); designer/others rejected. Reconciles chore-ownership canon: dev authors chores directly, PM files Features/Bugs.
- open / designate → **NO gate yet** (#1540 builds it; PM opens via manual-promote stopgap today).
- PM story-shape (use-case format + domain-purity) → **GATED at create**: pre-filing-audit Check 0.
- Dev positive work-shape (TDD / vertical-slice / outside-in) → **SKILL-enforced**, not hard-gated (test-before-impl ordering — realistic ceiling) — _[dev]_.
- Designer desk-check FIRING → **GATED** (designer-src-verified): `pass-desk-check` is designer-only across all 3 gate-classes (`workflow/bundled_default.rs:87/96/105`) + fires a 4-part stack at `tools_stories/acceptance/desk_check.rs:102-119` (role-allowed + designer-verdict-fresh ≤600s + CI-green `check_ci_green` + ATP-substantive); `fail-desk-check` = designer-or-PM (`bundled_default.rs:114`). The TRANSITION can't drift — only designer fires it, only on a fresh verdict + green CI.
- Designer review-COVERAGE (what to verify within the gate) → **SKILL-enforced** (desk-check-gate / pre-pickup-review / impl-leak-check / iteration-close-audit) — the gate fires the transition; the skills decide the verdict. _[designer]_.
- Designer board-SPEECH guard-hook → **DRAFTED/INERT** — open gap to close (sister to #1548) — _[designer]_.
- Acceptance acceptor-GATE (who fires, per class) + verdict-freshness → **GATED** (`workflow/bundled_default.rs:128-154`). **TEAM = the three agents (PM+dev+designer); owner is a SEPARATE axis, not part of "team".** owner-class→[ryan] (Ryan's walk alone); team-class→**PM fires on fresh dev+designer verdicts** (the three agents' autonomous call — does NOT wait on owner); chore→[dev]+designer evidence; verdicts EXPIRE (re-cast at ceremony-time). _The gate currently also lists [ryan] in team-class — the per-gate owner-stopgap; clean shape per owner-unrestricted = team→[pm] + owner via system-bypass (future owner-controls)._ PM-DRIVES-the-walk + team-silence + per-story pause-cadence → **SKILL-enforced** (`run-acceptance`), not hard-gated.
