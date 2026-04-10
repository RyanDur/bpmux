# CI Status UX Spec (#151)

*Designer spec — 2026-04-10. Builds on Ryan's story body + PM's edits around attribution-not-blame framing. Phase 1 only.*

## Problem

The only way any role learns CI failed today is if Ryan types "ci failed" into the chat. PM then runs `gh run list` manually. Designer and dev receive no signal at all — they may keep making decisions against a red main indefinitely.

CI state is a **shared fact** that every role should see. But **what to do about it** is a per-role decision: PM files, dev investigates, designer waits. The indicator is a notification surface, not a directive. The UX must honor both halves of that.

## Phase 1 Scope

- **Status bar indicator.** One element, fits in the existing `[role]  ● designer ● dev ● pm` status bar.
- **Three states.** ✓ green / ✗ red / ◌ running. No stale, no unknown yet — those are Phase 2.
- **Polling.** `gh run list -L 1` every 60 seconds via a background tokio task. Single run polled per tick, lowest-latency truth.
- **No drill-in.** Clicking the indicator does nothing in Phase 1. Details come from running `gh` in the terminal yourself.
- **No attribution text in the indicator.** The indicator shows state, not who broke it. Attribution UI is Phase 2 and requires the framing guidance below before shipping.
- **No notifications.** No toast, no beep, no system message broadcast. A flip from green to red is visible only as the indicator changing color. Deliberately quiet.

## UI Placement

The indicator sits at the right edge of the status bar, just before the keyboard hints:

```
 [all]  ● designer ● dev ● pm          CI ✓          ?=help ⌥1=sidebar Enter=send
```

When red:

```
 [all]  ● designer ● dev ● pm          CI ✗          ?=help ⌥1=sidebar Enter=send
```

When running:

```
 [all]  ● designer ● dev ● pm          CI ◌          ?=help ⌥1=sidebar Enter=send
```

**Why right-of-role-dots, not left:** the role presence cluster (`● designer ● dev ● pm`) is the primary live signal of "who is here." CI is secondary — it's repository state, not presence state. Right alignment separates the two cognitive groups visually.

**Why a single character glyph (✓ ✗ ◌), not a word:** the status bar is 80 columns wide and every element competes for space. Glyphs read instantly. Words (`CI PASS`, `CI FAIL`) eat horizontal budget that belongs to the chat pane.

**Why the literal text "CI" as a prefix:** without the label, a standalone ✓/✗ at the right edge could be read as "message delivered" or "typing indicator." `CI ✓` disambiguates.

## Colors

Ratatui style conventions:

- **Green** (`Color::Green`) — passing. Same green as status dots when idle.
- **Red** (`Color::Red`) — failing. Same red as urgent status.
- **Yellow** (`Color::Yellow`) — running / in progress. Distinct from both passing and failing.

**Accessibility note:** color alone is insufficient. The glyph differences (`✓` / `✗` / `◌`) carry redundant meaning for users with red/green color blindness. Both channels (color + shape) must change together.

## State Semantics

### Green (✓)
The most recent CI run on the default branch completed with `conclusion: success`.

### Red (✗)
The most recent CI run on the default branch completed with `conclusion` in `{failure, cancelled, timed_out, action_required, neutral}`. All non-success conclusions render red — we deliberately do not distinguish "failed" from "cancelled" in the indicator. Details belong in terminal `gh run view`.

### Running (◌)
A CI run on the default branch is `in_progress` or `queued`. If there are multiple concurrent runs, the indicator shows running while ANY is running, and reflects the latest COMPLETED run otherwise. Running wins over stale success or stale failure — the user cares that a fresh answer is coming.

### What is NOT a state in Phase 1
- **Stale** (more than N hours since last run). Phase 2.
- **Unknown** (API error, offline, no gh credentials). Phase 2.
- **Per-commit** (CI for HEAD of my local branch vs. default branch). Phase 2 / open question.

In Phase 1 if polling fails, the indicator shows the LAST KNOWN state without any error marker. The bpmux instance's own diag log captures polling errors via the diag-logging convention. Failed polls do not corrupt the frame.

## Per-Workspace Consistency (the critical design constraint)

All four workspaces (bpmux, bpmux-dev, bpmux-pm, bpmux-designer) point at the same bpmux repo. Each one runs its own poll; each one shows its own indicator. This is deliberate.

**The rule: same fact, independent views.**

- Every role's TUI shows the same CI state (because the fact is the same).
- Each role decides independently what to do about it.
- No role's action propagates to another role's TUI.
- If PM "acknowledges" a red main (whatever that might mean in a future story), that state is local to PM's workspace. Dev's TUI still shows red.

**Why no propagation:** acknowledgment propagation is a sneaky path to shared interpretation. "PM saw it, so dev can relax" is a collective mental model that belongs to the team, not to the individual agents. The whole point of the individuality principle is that each role's response to a fact is their own.

This constraint rules out several tempting Phase 2 features we should NOT build:
- "Seen by PM at 11:32" annotation visible in other workspaces
- Shared "who is investigating" badge
- Cross-role ownership claim

All of those collapse individual responses into a team view. They're wrong for this product.

## Attribution — Routing, Not Blame

Phase 1 does not ship any attribution UI. This section exists because Phase 2 probably will, and the framing must be baked in from the spec, not added as a review note later.

**The rule: attribution is routing information, not accountability theater.**

When Phase 2 adds drill-in details, the copy must follow these forms:

### Correct framing (routing)

```
CI ✗  main failed at 9af6c84
Last green: f5485f8 (Designer: Left arrow collapses section header)
Pushed by: dev
Suggested next action: ask dev about 9af6c84
```

Notice: "Pushed by" is neutral attribution. "Suggested next action" is routing — who would know most about the context.

### Forbidden framing (blame)

```
[FORBIDDEN]  dev broke the build at 9af6c84
[FORBIDDEN]  Responsible: dev
[FORBIDDEN]  9af6c84: @dev — your commit broke main
[FORBIDDEN]  dev has not yet fixed 9af6c84 (3 hours overdue)
```

Blame framings are forbidden because:
1. They assume causation from timing (recent commit + red main ≠ that commit's fault). The team demonstrated this exact error at ~12:35 UTC on 2026-04-10 when a test flake was blamed on a refactor commit that didn't touch the test.
2. They invite social loafing by creating a "responsible party" label that other roles can hide behind. Per transparency paradox research (Bernstein 2012), visible blame allocation degrades team performance.
3. They fight the individuality principle: each role's response to the fact is their own, not their reaction to being told they are the responsible party.

### UI copy test

Before shipping any attribution string, ask: "Would this phrasing make the named person feel called out in front of the team, even if CI passed on second look?" If yes, the phrasing is blame. Rephrase to routing.

## No Pile-On

Phase 1 does not ship any notification or broadcast. This is deliberate.

Today (pre-#151), CI failures surfaced via Ryan announcing them. That worked but triggered a pile-on: all three agents attempted to respond to the same fact simultaneously, creating the "converge-then-diverge" flip pattern that made the chat unreadable for Ryan. If #151's indicator were paired with a broadcast ("CI ✗ — main is red"), the same pile-on would happen automatically.

**Phase 1 rule: the indicator state is passive. No side effects beyond the pixel change.** A role can see the indicator go red and decide to say something in chat. A role can decide not to. The indicator does not itself generate chat.

Phase 2 / later stories may add subtle notifications (sound, toast on change). Before those ship they must address:
- How to prevent multi-role simultaneous reactions
- Whether notifications fire in all workspaces or only one
- Whether a "I'm handling this" marker would help (probably no, per the no-propagation rule above)

## Data Source

`gh run list -L 1 --json status,conclusion,headSha,displayTitle,workflowName,createdAt`

**Why `gh` CLI and not the REST API directly:**
- Uses the existing GitHub credentials Ryan already has configured
- No new auth layer to worry about
- `gh` already handles rate limiting gracefully (returns cached results, surfaces 429s)
- Rust calling out to a subprocess is simpler than re-implementing the REST auth

**Why `-L 1` (single run):**
- Lowest latency (smallest JSON body)
- Polling every 60s across 4 workspaces is 4 requests/min total — well under gh's rate limit
- The state transition we care about is "has the latest run changed?" which needs only the most recent

**Why 60-second polling:**
- Fast enough to be meaningful (red main for a minute is fine; red main for 10 minutes is a problem)
- Slow enough to keep rate-limit headroom for the rest of the team's gh usage
- Round number, easy to reason about

**Polling implementation:** background tokio task, same pattern as the existing `session::awareness::spawn`. Errors routed through `session::diag::log_diag` per the diag-logging convention — never to stdout/stderr.

## Non-Goals (Phase 1 and Forever)

Explicit list of things #151 will not build, even in later phases:

- **Auto-generated "responsible party" notifications.** See attribution section above.
- **Shared "acknowledged by" state.** See per-workspace consistency section above.
- **Cross-workspace "who is investigating" tracking.** Same reason.
- **Auto-retry UI.** Retrying a failed CI run is a `gh` action; the indicator doesn't own action verbs.
- **Inline failure logs.** The status bar is not for log content. `gh run view` in the terminal is the place for detail.
- **Pass/fail counters per person.** Explicit surveillance boundary per #42/#38 — no per-participant metrics, ever.

## Phase 2 Open Questions (not shipping in #151)

Documented here so the story doesn't accidentally fold them in:

1. Stale state (how long since last run before we flag it?)
2. Unknown state (how do we distinguish "offline" from "no runs yet"?)
3. Branch-aware state ("CI on my local branch" vs "CI on default branch")
4. Drill-in UI (clicking the indicator shows what? where?)
5. Notification-on-change (toast, sound, system message — with pile-on mitigations)
6. Multi-workflow aggregation (if the repo has multiple workflows, do we show worst-state or latest-state?)
7. Per-agent polling vs shared pollier (is 4x the polling of one-per-workspace worth it?)

These are Phase 2 stories, each drafted when the corresponding usage data arrives. Do not bundle them into #151.

## Acceptance Criteria

- **Given** the bpmux TUI is running with `gh` configured for the active repo
- **When** I look at the status bar
- **Then** I see a CI indicator showing the state of the most recent run on the default branch
- **And** the indicator updates within 60 seconds of a state change on GitHub
- **And** the indicator is identical across all four workspaces pointing at the same repo
- **And** the indicator's state change does not generate any chat message, toast, beep, or side effect in any workspace
- **And** the indicator shows the last known state if a poll fails, with the polling error logged via `session::diag::log_diag`
- **And** the glyphs are legible for red/green color blindness (shape differs across states, not just color)
- **And** no attribution text (who pushed, which commit) appears in Phase 1

## What NOT to build (concrete checklist for the implementer)

- [ ] NO message generation when state changes
- [ ] NO shared "acknowledged" state across workspaces
- [ ] NO per-commit attribution in the indicator
- [ ] NO blame framing anywhere in any copy string
- [ ] NO drill-in click handler (Phase 2)
- [ ] NO toast / beep / system message on change (Phase 2, with pile-on mitigations)
- [ ] NO counters, histories, or "who broke it" tallies (surveillance boundary)

## Related

- `#151` (this issue)
- `#106` — companion story (CI results in bpmux channel). Must also guard against pile-on. Designer comment on #106 already flags this.
- `#146` — green-dot-while-blocked presence bug. Same family of problems: status signals that don't reflect reality.
- `docs/diag-logging-convention.md` — the polling error-handling rule.
- `feedback_doc_volume_without_demand.md` (my memory) — the discipline check that said this spec passes the reader test.
