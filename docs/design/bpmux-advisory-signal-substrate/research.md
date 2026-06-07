# Research: how should bpmux advisory-signal possible dangers without prescribing teams' processes?

Filed under #1346 research-spike per Ryan 2026-05-15 01:35 UTC ("we could find a way to signal possible dangers... larger conversation backed by research") + 01:40 UTC ("do the research spike in here"). Pre-decomposition deliverable; informs Cassiopeia+ Epic-shape ratification.

**Authoring**: designer §2-§3 (design-space + cross-tool sister-pattern + affordance-shape), PM §1 (motivation) + §4-§5 (decomposition + cost). Dev welcomed on §3.X substrate-axis. Sister to `docs/design/agent-rooms-as-homes-architecture/research.md` co-authoring pattern.

---

## §1 Motivation (PM-authored)

### 1.1 Empirical trigger — Cassiopeia 4-Bug retire (2026-05-15)

Cassiopeia ratification ceremony 2026-05-15 01:10 UTC filed 4 substrate-foundation Bugs intended to close today's Pegasus 4-day discipline-gap (13/15 rejected) via substrate-enforce > discipline-only architectural-fix:

- **#1340** deliver_story grep-at-gate (catches inline-`//` absolute-rule violations)
- **#1341** edit_story field-completeness-test (catches decoder selective-field-drop)
- **#1342** falsifier-fidelity declaration at deliver_story (substrate-promptable class)
- **#1343** create_story living-doc clause for RTL-class (substrate-promptable class)

All 4 active-blocked at bpmux tool-runtime. Ryan caught architectural overreach 2026-05-15 01:35 UTC: "this tool needs for whomever is using it. not just this team. we should not prescribe anothers process." All 4 retired-as-superseded.

### 1.2 Substrate-axis distinction (root-cause)

The Pegasus retro convergence on `feedback_substrate_enforce_over_discipline_only` was correct at concept-level (discipline-fails-under-pressure → substrate-encode the discipline) but PM filed the gates at the WRONG substrate-layer:

- **Correct layer for THIS team's discipline**: this-team's bpmux-repo tooling (pre-commit hook / CI workflow / Claude Code plugin / bpmux-internal cargo-test) — filed at #1345 as Cassiopeia Wave-A Chore
- **Wrong layer**: bpmux-the-app substrate (prescribes process on other teams using bpmux)

The lesson saved (`feedback_bpmux_is_generic_team_coordination_not_team_process`): layer-discriminate at filing-time; bpmux MAY signal advisory dangers but cannot active-block on team-internal-discipline outside its workflow-domain.

### 1.3 Open architectural question (this research-spike's scope)

Ryan-canon 01:35 UTC: "we could find a way to signal possible dangers. That deserves some more thinking beyond some bugs." Signaling dangers WITHOUT prescribing process is a substrate-shape question. This research-doc surveys the design-space before any Bug-class implementation commits.

### 1.4 Sister-substrate-evolution signals (research-resurfacing-conditions)

Existing bpmux passive-signal substrate already in production (informs §3 cross-tool analysis):

- **`#1146` rework_comment ↩ glyph + tooltip on story-row** — passive-signal for "story has unresolved rework concern"; doesn't block any action; sister-pattern for §3.9 (a) inline-row badge
- **Story-state column-bucketing** — passive-signal via spatial-grouping (ready / in-progress / desk-check / delivered) reveals workflow-shape; sister to §3.9 (b) column-header summary
- **`#1258 r3` close_iteration ready-empty guard** — active-block protecting bpmux's OWN workflow-coherence (ceremony invariant); sister to dev 01:41 UTC boundary-discriminator framing

These existing substrate-pieces validate the passive-vs-active spectrum (§2.3) empirically. The research-question is: should bpmux EXPAND advisory-signal substrate to cover the §2.1 (a)-(g) danger-classes, and if so, at what shape?

### 1.5 Stakeholder-alignment shape

Per Ryan canon: bpmux serves whoever uses it. Passive-signal expansion must be:
- Default-on for SUBSTRATE-COHERENCE dangers (e.g., circular epic-membership) — these threaten data integrity for ANY user
- Default-off for TEAM-DISCIPLINE dangers (e.g., living-doc / falsifier-fidelity / no-comments) — these are team-process-prescription
- Opt-in for ADVISORY-CONVENIENCE dangers (e.g., long-pending desk-check / silent-role) — teams configure if useful

This stratification (data-integrity / team-process / advisory-convenience) is the substrate-axis the implementation must honor.

---

## §2 Design-space survey (designer-authored)

### 2.1 Dangers within bpmux's domain

bpmux's user-job is team-coordination: stories, channels, messages, accept-walk, iteration-ceremony. Candidate-dangers OBSERVABLE from within this substrate:

**(a) Story-state class** — observable from canonical-store + workflow-state transitions:
- Story stuck in `delivered` state for >N days (signals stale desk-check; possible reviewer-overload)
- Ready-queue empty during in-progress iteration (signals planning-gap; iteration-burn)
- Story lacks `## Acceptance test path` section (catches PM filing-discipline gap; sister to today's empirical ATP-rejection class)
- Circular epic-membership (story A.parent = B, B.parent = A; substrate-corruption)
- Story has no commits referencing its number (signals impl-stall; dev hasn't started)

**(b) Message-flow class** — observable from message-log + role-presence:
- Silent role (no broadcast for >N hours during in-progress story; signals stuck-or-blocked)
- Permission-prompt-class messages without owner-response (sister to today's #1325 permission-prompt-in-room class)
- Repeated send_message failures from same role (sister to today's api_error pattern)

**(c) Iteration-ceremony class** — observable from ceremony-tools:
- close_iteration on non-empty ready (today's #1258 r3 guard — ALREADY active-block; sister to advisory-signal class but at different position on the passive-vs-active spectrum)
- Empty close-ceremony (close fires with zero member-stories; signals premature-close)
- Multi-iteration overlap-collision (markers crossing positions in icebox_unified)

**(d) Workflow-state class** — observable from state-transition history:
- State-transition backward (reject_story Delivered→Ready; fail_desk_check DeskCheck→InProgress; sister to existing #1146 rework_comment ↩ glyph + tooltip — already passive-signal)
- Long-pending desk-check (>N hours since deliver_story; signals reviewer-overload OR delivery-quality-issue)

**(e) Convergence class** — observable across stories + sessions:
- Sister-shape divergence across stories (same Epic, different falsifier-shape; signals pre-pickup-review gap)
- Recidivism-class accumulation (lessons saved but apply-gap surfaces N times; signals canon-load-vs-application drift)

**(f) Substrate-consistency class** (dev contribution 2026-05-15 01:41 UTC) — observable from substrate-state divergence:
- State-snapshot drift between renderer + canonical store + GH (class of today's body-cascade-fix 5ae7266a empirical case)
- Half-state on partial mutation (class of BoardEvent fire-ordering pre-#1282 substrate)
- Substrate-config-drift (class of today's BPMUX_STORAGE_BACKEND env-var-race fix cc43baa)
- These ARE bpmux-domain: bpmux observes them; team-coordination depends on the consistency

**(g) PM-coordination class** (PM contribution 2026-05-15 01:41 UTC) — observable from coordination-shape:
- Scope-bloat per iteration (Pegasus 15+ stories empirical; signals filing-discipline gap)
- Team-divergence across roles (single story has 3 different shape-claims; signals pre-pickup-review gap)
- Filing-recidivism class (substrate-foundation Bugs as quick-fix to research-questions; today's empirical 4-Bug retire case)
- Silent-role accumulation (sister to message-flow class (b))
- Holding-pattern + permission-prompt-class messaging (sister to message-flow class (b))

### 2.2 NOT in bpmux's domain (other-team-tooling)

These belong in role-substrate or repo-tooling, NOT bpmux:
- Code-comments / lint-rules / language-syntax (other-team's editor / linter / pre-commit hook)
- Test-discipline / falsifier-fidelity (other-team's CI / cargo-test / vitest setup)
- Commit-message-shape / git-workflow (other-team's git hooks / commit-template)
- Build-tool output (cargo / tsc / npm — other-team's build-pipeline)
- File-paths / module-paths / type-names (other-team's code-structure)

**Boundary heuristic**: if the danger requires bpmux to understand the team's code, build-tools, language-syntax, or filesystem layout, it's NOT bpmux's domain. If the danger is observable from canonical-team-coordination-state (stories / messages / workflow-events / roles), it IS bpmux's domain.

### 2.3 Passive-signal vs active-signal shape

Two shapes at the affordance-axis:

**Passive (warn-on-display)**:
- bpmux renders the signal in a non-blocking surface (badge / hover-tooltip / activity-log entry / state-snapshot-message)
- User-action is NOT prevented; user is INFORMED + chooses
- Sister to existing `rework_comment` ↩ glyph + tooltip (today's #1146 substrate — already passive-signal pattern)
- Sister to GH issue labels surfacing context without blocking transitions

**Active (refuse-on-action)**:
- bpmux tool returns structured error preventing the action
- User-action is BLOCKED until danger resolved
- Sister to today's `#1258 r3 ready-empty guard` on close_iteration (already active-block)
- Sister to `feedback_bpmux_no_manual_status` hook (active-block on manual set_status)

**Designer-lens recommendation** (sharpened per dev 01:41 UTC + PM 01:41 UTC convergence): PASSIVE-SIGNAL DEFAULT per Ryan 01:35 UTC framing ("advisory-signal"). Active-signal JUSTIFIED when protecting bpmux's OWN workflow-ceremony coherence (story state-machine / iteration ceremony / channel scoping / substrate-consistency).

**Decision-rule** (dev 01:41 UTC framing): "Is this protecting bpmux's OWN workflow-domain coherence?" → active OK. "Is this enforcing the team's process beyond bpmux's workflow-domain?" → passive only.

The distinction matters: active-blocking on team-process violations = prescribing process (today's retired 4-Bug overreach class); active-blocking on bpmux-workflow-coherence = preserving substrate-integrity (today's #1258 r3 close_iteration ready-empty guard is JUSTIFIED on this rule).

**Spectrum of escalation** (sister to today's `feedback_3_state_ux_for_permission_prompts` ambient → elevated → critical pattern):

| State | Trigger | Affordance | Sister-pattern in bpmux |
|---|---|---|---|
| **Ambient** | Danger detected | Inline badge / tooltip / activity-log line; user-aware-not-blocked | Existing rework_comment ↩ glyph |
| **Elevated** | Danger persists / repeats | Renderer attention-cue (color-shift / pulse / sort-priority); still non-blocking | Possible new pattern; sister to permission-prompt §2.6 |
| **Critical** | Substrate-corruption-risk | Active-block + named-cause error | Existing #1258 r3 close-iteration guard |

User can override Ambient + Elevated; Critical requires resolving danger first.

---

## §3 Cross-tool sister-pattern (designer-authored)

### 3.1 Linear

- **Signal-shape**: badges + sort-priorities (overdue / blocked / no-estimate)
- **Active-blocking**: rare; configured per-workspace (e.g., required-fields-on-create)
- **Default**: PASSIVE-signal majority
- **Override**: user-can-override most signals; admin-config for required-fields
- **Sister-applicability for bpmux**: badges on story-rows in renderer (sister to existing story-state column-bucketing)

### 3.2 Jira

- **Signal-shape**: warnings panels + workflow-validators + reporters
- **Active-blocking**: extensive, per-project-configurable (workflow conditions / validators / post-functions)
- **Default**: PASSIVE-warnings; teams opt-in to active-validators
- **Override**: admin-defines per-workflow; users can request exception
- **Sister-applicability for bpmux**: per-Epic optional active-validators (e.g., "this Epic requires ATP-path-present") — opt-in shape; doesn't prescribe across teams

### 3.3 GitHub

- **Signal-shape**: status-checks + branch-protection-rules + required-reviewers + CODEOWNERS
- **Active-blocking**: per-branch-protection (admin-configurable); status-checks can be required or advisory
- **Default**: ADVISORY-status-checks; teams opt-in to required-checks
- **Override**: admin-grants bypass (PR-merge-protection-overrides); user-action via "merge anyway" if not required
- **Sister-applicability for bpmux**: per-iteration optional gates (e.g., "this iteration requires CI-green on accept_story") — opt-in shape

### 3.4 Asana

- **Signal-shape**: due-date warnings + dependency-overdue badges + sort-priorities
- **Active-blocking**: rare; mostly advisory
- **Default**: PASSIVE-warnings; "incomplete dependency" warns but doesn't block
- **Override**: user-can-override
- **Sister-applicability for bpmux**: passive-signal-only shape

### 3.5 Notion

- **Signal-shape**: limited; pure-document substrate
- **Active-blocking**: none beyond schema (required-fields on database entries)
- **Default**: NO danger-signaling beyond schema
- **Sister-applicability for bpmux**: not informative for danger-signaling question

### 3.6 Trello

- **Signal-shape**: due-date warnings + power-up extensions
- **Active-blocking**: rare; power-up-dependent
- **Default**: PASSIVE-warnings via power-up ecosystem
- **Sister-applicability for bpmux**: extension-ecosystem shape (Claude Code plugins serve similar role per Ryan 2026-05-15 01:35 UTC framing)

### 3.7 Pivotal Tracker (legacy reference)

- **Signal-shape**: velocity-based projections + "story too big" warnings + iteration-overflow indicators + ACCEPTANCE-CRITERIA-REQUIRED on stories
- **Active-blocking**: minimal (mostly); acceptance-criteria gate is exception
- **Default**: PASSIVE-advisory + auto-decomposition-suggestions
- **Sister-applicability for bpmux**: velocity-substrate sister (today's Epic #1236 candidate); iteration-burn signal (sister to today's `feedback_iteration_burn_signals_tool_gap`); acceptance-criteria-required pattern is bpmux's own ATP-required gate (already shipped)

### 3.7.5 Shortcut / Clubhouse (PM contribution 2026-05-15 01:41 UTC)

- **Signal-shape**: soft-warning badges on stories; team-velocity projection
- **Active-blocking**: minimal
- **Default**: PASSIVE-warnings
- **Sister-applicability for bpmux**: convergent industry-pattern

### 3.7.6 Basecamp (PM contribution 2026-05-15 01:41 UTC)

- **Signal-shape**: no danger-signaling; pure-discussion substrate
- **Active-blocking**: none
- **Sister-applicability for bpmux**: outlier; confirms passive-signal is industry-default (zero-signal is the floor)

### 3.8 Cross-tool convergence

**Pattern**: industry-default is PASSIVE-signal + USER-OPT-IN to active. Active-blocking is reserved for either (a) substrate-corruption-class (data-integrity) OR (b) workspace-admin-configured per-workflow rule (team-explicit-opt-in).

**Implication for bpmux**: passive-signal-default + opt-in-active for narrow classes. Default-shape MUST NOT prescribe other teams' processes; opt-in shape allows teams to bake their own discipline if they choose.

### 3.9 Affordance-shape if advisory implemented

**UX surfaces in bpmux renderer** (per existing substrate):

(a) **Inline-row badge** — sister to existing `rework_comment ↩` glyph on story-rows
- Trigger: danger-class observable from story-state
- Render: small icon + tooltip on story-row
- User-action: hover for context; click for detail-pane
- Cost: low (inline-CSS + icon-set)

(b) **Column-header summary** — count of danger-class items per column
- Trigger: aggregate-count of stories in column with signals
- Render: count-badge next to column-name
- User-action: filter-by-danger / sort-by-danger
- Cost: low (existing column-render substrate)

(c) **Activity-log message** — bpmux posts a system-message into the appropriate channel when danger detected
- Trigger: state-transition event matching danger-class
- Render: system-class message in chat
- User-action: read + decide; reply with action-or-acknowledge
- Cost: medium (BoardEvent + message-broadcast)
- Sister to existing state-delta broadcast pattern

(d) **State-snapshot panel** — periodic summary of all active dangers across the board
- Trigger: on-board-load OR on-iteration-ceremony OR explicit-tool-call
- Render: top-panel collapsible section
- User-action: review + dismiss / act
- Cost: higher (aggregation substrate + UI-surface)

**Affordance-decision criteria**:
- Per-story dangers → (a) inline-row badge
- Per-column / per-iteration dangers → (b) column-header summary
- Time-class dangers (silent role / long-pending desk-check) → (c) activity-log message
- Cross-substrate convergence → (d) state-snapshot panel

**A11y substrate** (sister to today's a11y-audit discipline):
- Badges use both color + glyph (color-blind-safe)
- ARIA `role="status"` for ambient signals; `role="alert"` only for critical
- Keyboard-navigability: signals don't trap focus; affordances reachable via Tab
- Sister to existing rework_comment a11y treatment

### 3.10 Boundary-respect discipline

Per Ryan 01:35 UTC "don't prescribe another's process":

**bpmux ships passive-signal substrate** for dangers in §2.1 (a)-(e) — observable from canonical team-coordination state.
**bpmux does NOT ship active-blocking** beyond existing substrate-corruption-guards (#1258 r3 + atomic-fire patterns).
**bpmux ships OPT-IN active-validators** at per-workspace/per-iteration scope (sister to Jira/GH model) — teams choose to enable.

This preserves Ryan's "whoever uses bpmux" canon: defaults don't prescribe; opt-ins are team-choice.

---

## §4 Decomposition (PM-authored)

### 4.1 Decomposition principle

Per `feedback_yagni_deferral_names_resurfacing_condition` — research-doc lands; team converses on §6 owner-recommendation; if convergence achieves shape, file follow-up Feature(s) for next iteration. NOT this iteration (Cassiopeia stays focused on iteration-substrate pivot + reworks).

Per §1.5 stakeholder-alignment shape — three default-stratification classes drive decomposition.

### 4.2 Three-class decomposition

**Class-A: Substrate-coherence dangers (default-on, active-block)**
- These threaten data integrity regardless of team — circular epic-membership / half-state on mutation / state-snapshot drift / substrate-config-drift
- Already partially-shipped (#1258 r3 + atomic-fire patterns + cascade-fix 5ae7266a + storage_tests env-var-mutex)
- Decomposition: enumerate remaining Class-A dangers (§2.1 (a) circular epic / (d) state-transition backward IF integrity-class / (f) substrate-consistency) + file as Bug-class per `feedback_substrate_enforce_over_discipline_only` substrate-fix pattern
- ~3-5 stories estimate

**Class-B: Team-process dangers (default-off, opt-in active)**
- These prescribe team-discipline if active — living-doc / falsifier-fidelity / no-comments / RTL-discipline
- DO NOT IMPLEMENT in bpmux substrate. These belong in this-team's tooling layer (#1345 Chore covers).
- Decomposition: ZERO new bpmux stories. Refer to #1345 + future team-tooling Chores for THIS team's discipline-substrate.
- If a future-team-using-bpmux wants similar discipline, they fork or contribute to claude-plugin / repo-tooling layer.

**Class-C: Advisory-convenience dangers (default-off OR default-on-passive, opt-in shape)**
- These advise without prescribing — long-pending desk-check / silent-role / scope-bloat / filing-recidivism class
- Default-on PASSIVE-signal (badge / tooltip) safe per Ryan-canon ("advisory")
- Opt-in ACTIVE-signal via per-workspace/per-iteration config (sister to Jira / GitHub model)
- Decomposition: substrate-primitive Slice 1 (signal-data-model) → renderer-primitive Slice 2 (badge + tooltip component) → archetype implementation Slice 3 (pick one Class-C danger as proof) → sister-pattern replication Slices 4-N
- ~4-8 stories estimate (substrate + renderer primitive + 2-6 danger-class implementations)

### 4.3 Class-A enumeration (this research-doc's substrate-coherence-Bug candidates)

Concrete Bug-candidates for follow-up filing (when team converges):

(i) **Circular epic-membership detection** — set_parent_epic guards against cycle (A.parent=B + B.parent=A); structured error
(ii) **Sister-shape divergence at pre-pickup** — pre_pickup_review tool detects Epic-children with diverged falsifier-shapes; reports for cross-role review
(iii) **Half-state guard on atomic-fire** — extend atomic-fire substrate (sister to #1300 close_iteration positional) to surface partial-failure with named-cause
(iv) **State-snapshot drift detector** — periodic check renderer ↔ canonical-store ↔ GH consistency; alert via system-message on divergence
(v) **Substrate-config-drift detector** — env-var mismatches between expected + actual

### 4.4 Class-C enumeration (advisory-convenience archetypes)

Initial passive-signal archetypes (default-on):

(i) **Story stuck in delivered >N days** — yellow-badge + tooltip + hover-details
(ii) **Silent role >N hours during in-progress** — system-message-class entry in role's-channel; passive-signal
(iii) **Long-pending desk-check >N hours** — badge on story + designer-channel ping
(iv) **Scope-bloat per iteration warning** — iteration-marker color-shift when N stories committed exceeds team-velocity-historical-mean
(v) **Filing-recidivism convergence** — system-message when N+ stories file matching pattern (e.g., substrate-foundation Bug class within 1 hour)

Default-off (opt-in active) candidates:
(vi) **Story lacks ATP path** — active-block at deliver_story (today's bpmux already enforces this; sister-pattern reference)
(vii) **Pause-vocabulary without named-cause** — passive-signal default; opt-in active per team-discipline
(viii) **Permission-prompt-class messages** — passive-signal default; opt-in active per team-discipline

### 4.5 Cross-iteration sequencing

Per §1.5 + Ryan canon: ship Class-A subset within current/next iteration (substrate-foundation Bug class for bpmux's OWN coherence). Defer Class-C archetypes to follow-up iteration AFTER team-converse + cross-role concur on first archetype shape.

Estimated total scope if all Class-A + Class-C archetypes implement: 7-13 stories across 2-3 iterations.

---

## §5 Cost/complexity-to-impl (PM-authored)

### 5.1 Substrate-primitive cost

**Signal-data-model** (Class-C Slice 1): canonical-store extension with DangerClass enum + Severity (Ambient/Elevated/Critical) + Source (story/iteration/channel/role) + ObservedAt + DismissedBy
- Cost: 1 small story (Rust + TS schema parity per #1298 sister-substrate pattern)
- Risk: low (additive substrate)

**BoardEvent integration**: new event class `DangerDetected { class, severity, source, payload }` + sister `DangerDismissed`
- Cost: 1 small story (sister to BoardEvent::StateTransitioned pattern; #1282 substrate-foundation)
- Risk: low (existing event-substrate)

### 5.2 Renderer-primitive cost

**Generic danger-badge component** (Class-C Slice 2): React component renders badge + tooltip + a11y substrate per §3.9 affordance
- Cost: 1 medium story (designer UX-axis + dev impl)
- Risk: medium (a11y substrate; sister to existing rework_comment treatment)
- Reusability: substrate consumed by ALL Class-A + Class-C signal types

**Activity-log message-class integration**: bpmux posts system-message-class for danger-class events
- Cost: 1 small story (sister to existing state-delta broadcast)
- Risk: low

### 5.3 Per-danger-class implementation cost (Class-C archetypes)

Each Class-C archetype (Slices 3+) following the same shape:
- Detector logic (substrate-side observation rule)
- Trigger integration (existing BoardEvent listener OR periodic check)
- Renderer surface (badge / tooltip / system-message)

Cost per archetype: ~1 small-medium story (assumes primitive substrate-foundation shipped)

Total Class-C scope: 4-8 stories (substrate-primitives + 2-6 archetype implementations + dismiss-UX)

### 5.4 Class-A scope cost

Class-A substrate-coherence Bug-candidates per §4.3 (i-v):

Each Class-A Bug: ~1 small story (active-block at substrate level; pattern shipped today across #1258 + #1300 + cascade-fix)

Total Class-A scope: 3-5 stories

### 5.5 Opt-in active-signal cost (Class-C escalation)

Per-workspace/per-iteration config substrate (sister to Jira workflow-validators / GitHub branch-protection):
- Configuration substrate (key-value store for opt-in flags) — 1 medium story
- Tool-runtime config-check at deliver_story / accept_story / sister-tools — 1 small story
- Renderer UI for opt-in toggle (per-team settings) — 1 small-medium story

Total opt-in scope: 3 stories (post-passive-default ships; not iteration-1)

### 5.6 UX-cost (designer-axis)

Affordance-shape design per §3.9 (a)-(d):
- (a) inline-row badge: low UX-cost (sister to rework_comment ↩)
- (b) column-header summary: low UX-cost (existing column-render substrate)
- (c) activity-log message: low UX-cost (existing message-substrate)
- (d) state-snapshot panel: medium UX-cost (new top-panel UI surface)

A11y review per affordance: ~per-story (designer-axis pass before deliver)

### 5.7 Resurfacing-condition for follow-up filings

Per `feedback_yagni_deferral_names_resurfacing_condition` — defer with named resurfacing-condition.

**Trigger to fire Class-A follow-up filings** (Bug-class — substrate-coherence):
1. **Empirical convergence**: 2+ instances of substrate-coherence violation observed in dog-fooding OR retro discussions
2. **Cross-role concur**: PM + designer + dev align on shape of substrate-coherence-guard
3. **Bpmux-OWN-workflow boundary check**: danger is bpmux-substrate-integrity class, NOT team-discipline class

**Trigger to fire Class-C follow-up filings** (Feature-class — passive-advisory archetype):
1. **3+ instances of empirical-team-pain** matching a Class-C danger-class observed in dog-fooding (sister to designer §6 trigger)
2. **Cross-role concur** on passive-signal shape AND default-on/default-off + opt-in axis decision
3. **Boundary discriminator PASS**: danger is observable from canonical team-coordination state (§2.2 boundary-heuristic)

**Anti-trigger** (do NOT file):
- Retro-pressure to "close a discipline-gap quickly" — sister to today's empirical 4-Bug overreach class
- Team-internal-discipline pain (lives in this-team's tooling layer per #1345, not bpmux)
- Pre-mature filing before team-converse on substrate-shape

### 5.8 Iteration-budget shape

If Class-A + Class-C archetypes ship across 2-3 iterations:

**Iteration A (post-Cassiopeia ratification)**: Class-A substrate-coherence Bugs (3-5 stories) — bpmux's OWN workflow-coherence

**Iteration B**: Class-C substrate-primitive + 2 archetypes (4-5 stories) — first passive-advisory shape

**Iteration C** (deferred per resurfacing-condition): additional Class-C archetypes + opt-in active config (4-6 stories)

Total budget: 11-16 stories across 2-3 iterations; assumes resurfacing-conditions met.

---

## §6 Owner-recommendation (cross-role draft)

### 6.1 Synthesis

The research surfaces a clean THREE-CLASS stratification (§1.5 + §4.2) that resolves Ryan's "advisory-signal not prescription" framing into actionable substrate-shape:

- **Class-A (substrate-coherence)**: default-on, active-block. Protects bpmux's OWN workflow-domain coherence regardless of team. NOT prescription — data-integrity for ANY user. Empirical proof: existing `#1258 r3` close-iteration ready-empty guard + atomic-fire patterns are already shipped Class-A.
- **Class-B (team-process)**: default-off, do-NOT-implement in bpmux. Lives in this-team's tooling layer (#1345 Chore) OR Claude-plugin contribution layer. Prescriptive on other teams' processes if shipped in bpmux substrate.
- **Class-C (advisory-convenience)**: default-on-passive, opt-in-active. Industry-default shape (per §3 cross-tool sister-pattern survey). bpmux-domain dangers observable from canonical team-coordination state; teams CHOOSE to escalate to active if useful.

The 4 retired Wave-1 Bugs (today's empirical overreach) MISCLASSIFIED Class-B as Class-A: they treated team-internal-discipline as if it were substrate-integrity. The research clarifies the boundary.

### 6.2 Recommendation by class

**Class-A — recommend SHIP** (Iteration B per §5.8 sequencing; post-Cassiopeia ratification):

File 3-5 Bug-class stories for substrate-coherence dangers per §4.3 enumeration. Active-block at substrate-level for these is JUSTIFIED per dev 01:41 UTC decision-rule: "Is this protecting bpmux's OWN workflow-domain coherence?" → active OK.

Concrete candidates ratified by research:
- Circular epic-membership detection (set_parent_epic cycle guard)
- Sister-shape divergence at pre-pickup (Epic-children falsifier-divergence detector)
- Half-state guard extension across atomic-fire substrate
- State-snapshot drift detector (renderer ↔ canonical-store ↔ GH consistency)
- Substrate-config-drift detector (env-var mismatches)

**Class-B — recommend DO NOT SHIP in bpmux substrate**:

Zero new bpmux stories. Today's empirical 4-Bug retire-class confirms this boundary. Team-internal-discipline (no-comments / falsifier-fidelity / living-doc / RTL-specific patterns) MIGRATES to:
- `#1345` this-team's bpmux-repo tooling (pre-commit hook + CI + cargo-test invariant)
- Future Claude-plugin contribution layer for opt-in agent-action-time enforcement (sister to existing claude-plugin/ directory)

**Class-C — recommend DEFER + RESURFACING-CONDITION** (Iteration C per §5.8):

Defer Class-C implementation to follow-up iteration AFTER team-converse on first-archetype shape. Per `feedback_yagni_deferral_names_resurfacing_condition` resurfacing-conditions named in §5.7. Pre-mature filing risks today's overreach-class recurrence.

WHEN resurfacing-condition triggers, ship Class-C as:
- Slice 1: substrate-primitive (signal-data-model + BoardEvent::DangerDetected/Dismissed)
- Slice 2: renderer-primitive (generic danger-badge component + activity-log integration)
- Slice 3+: archetype implementation per Class-C danger (one per Slice; sister-shape replication)

### 6.3 Default-shape stratification (load-bearing per Ryan-canon)

**Default-on + active** — Class-A ONLY (bpmux-substrate-integrity):
- Justification: protects data integrity for ANY user; no team-process prescription
- Examples shipped: `#1258 r3` close-iteration ready-empty + atomic-fire guards + cascade-fix

**Default-on + passive** — Class-C bpmux-domain conveniences:
- Justification: surfaces team-coordination-pain without prescribing action; user chooses
- Examples to ship (post-resurfacing): stuck-delivered badge / silent-role activity-log / scope-bloat marker-color

**Default-off** — Class-B team-process OR Class-C opt-in-active:
- Justification: teams opt-in to discipline that THIS team finds useful but others may not want
- Examples: pause-vocabulary active-block / no-comments deliver_story-block / falsifier-fidelity declaration-required

### 6.4 Boundary-respect discipline (recursive guard)

Per Ryan-canon "we should not prescribe another's process" — any future Class-C filing MUST PASS:

1. **Boundary-heuristic test** (§2.2): is the danger observable from canonical team-coordination state? → Class-C eligible. Requires bpmux to understand code / build / language? → reject as Class-B.
2. **Default-shape stratification check** (§6.3): is the default safe for ANY bpmux-user team? → default-on. Does the default prescribe team-process? → default-off (opt-in).
3. **Anti-trigger check** (§5.7): is this filing under retro-pressure to close a discipline-gap quickly? → reject; revisit after team-converse.

Sister-discipline: substrate-axis-discrimination check at filing-time. Sister to `feedback_substrate_enforce_over_discipline_only` (sharpened post today's retire-lesson) — substrate-enforce is right at substrate-LAYER, wrong at substrate-AXIS when applied to team-process.

### 6.5 Owner-decision points (Ryan-ratify candidates)

(a) **Ship Class-A candidates** per §4.3 enumeration as post-Cassiopeia Bug-batch? (Recommend YES; concrete substrate-coherence-protection)

(b) **Defer Class-C** to Iteration C-or-later pending resurfacing-condition (§5.7)? (Recommend YES; honors "larger conversation backed by research" canon)

(c) **Confirm Class-B boundary-respect** — bpmux NEVER ships team-process active-blocks beyond bpmux's own workflow-domain? (Recommend YES; today's empirical overreach class)

(d) **Architectural-canon promotion** — promote 3-class stratification + boundary-heuristic + decision-rule to durable team-canon (saveable lesson)? (Recommend YES; sister to `feedback_bpmux_is_generic_team_coordination_not_team_process`)

### 6.6 Cross-role concur

- **Designer (this-author)**: §6.2 + §6.3 + §6.4 ratified per UX-axis (passive-default + opt-in-active = industry-pattern + Ryan-canon-alignment).
- **PM**: §1 + §4 + §5 authored; §6 concur pending review.
- **Dev**: §2.1 (f) + §2.3 decision-rule contributions absorbed; §6 substrate-axis review deferred per Ryan 01:41 UTC "don't distract the dev." Dev-engagement at implementation-filing time per resurfacing-condition.

---

## §7 Resurfacing-conditions summary (PM-ratified)

Per `feedback_yagni_deferral_names_resurfacing_condition` — concrete triggers for follow-up filings:

**Fire Class-A Bug-batch** when ALL true:
- 2+ empirical instances of substrate-coherence violation observed
- Cross-role concur on shape
- Bpmux-OWN-workflow boundary check passes

**Fire Class-C primitive-substrate + archetype-Feature** when ALL true:
- 3+ empirical instances of team-coordination-pain matching a Class-C danger
- Cross-role concur on passive-signal shape + default-axis decision
- Boundary-heuristic PASSes (§2.2 observable-from-coordination-state)

**Fire Class-C opt-in-active escalation** when ALL true:
- Class-C passive-substrate shipped + N teams empirically requesting opt-in
- Cross-role concur on opt-in-config substrate shape
- Per-workspace-config-axis defined (not single-team-bias)

**DO NOT fire (anti-triggers)**:
- Retro-pressure to "close discipline-gap quickly" — today's 4-Bug overreach class lesson
- Team-internal-discipline pain (→ #1345 team-tooling layer instead)
- Pre-converge filing without team-discussion on substrate-shape

*Pending team-converge + Ryan-ratify. Anticipated shape: passive-signal-default for narrow bpmux-domain danger-classes (§2.1) + opt-in-active substrate for advanced configurations; defer implementation filings to follow-up iteration per `feedback_yagni_deferral_names_resurfacing_condition`.*

---

## Cross-references

- `#1346` parent research-spike
- `docs/design/agent-rooms-as-homes-architecture/research.md` — sister research-doc pattern
- Ryan-canon 2026-05-15 01:35 UTC: "we should not prescribe another's process" + "larger conversation backed by research"
- Ryan-canon 2026-05-15 01:40 UTC: "do the research spike in here"
- Pegasus retro: substrate-enforce-over-discipline-only convergence (which led to the 4-Bug overreach this research interrogates)
- `feedback_yagni_deferral_names_resurfacing_condition` — defer with named resurfacing-condition
- `feedback_substrate_enforce_over_discipline_only` — substrate-fix architectural-pattern (now scoped to THIS-team's substrate per Ryan reframe)
- Existing passive-signal patterns: `#1146` rework_comment ↩ glyph; story-row state badges
- Existing active-block patterns: `#1258 r3` close_iteration ready-empty guard; `feedback_bpmux_no_manual_status` hook

---

## Authoring status

- §1 Motivation: ✓ PM (2026-05-15 01:45 UTC)
- §2 Design-space survey: ✓ designer (2026-05-15 01:42 UTC) + dev/PM contributions absorbed
- §3 Cross-tool sister-pattern + Affordance-shape: ✓ designer (2026-05-15 01:42 UTC) + PM sister-tool additions
- §4 Decomposition: ✓ PM (2026-05-15 01:45 UTC)
- §5 Cost/complexity: ✓ PM (2026-05-15 01:45 UTC)
- §6 Owner-recommendation: ✓ designer draft (2026-05-15 01:46 UTC); PM concur pending; Ryan-ratify pending
- §7 Resurfacing-conditions: ✓ PM-ratified (2026-05-15 01:46 UTC)

Open questions for team-converge:
- (i) Does §2.2 boundary-heuristic land cleanly? Any candidate-dangers missed?
- (ii) Does §2.3 passive/active spectrum + §3.10 boundary-respect-discipline align with Ryan's "advisory-signal" intent?
- (iii) Cross-tool sister-pattern §3 — any tool to add (custom-built; in-house tools)?
- (iv) §3.9 affordance-shape (a)-(d) — preferred default-shape for first implementation if/when team-converges to file Feature?
