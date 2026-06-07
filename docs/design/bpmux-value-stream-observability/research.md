# Research: how should bpmux visually communicate a team's process — bottlenecks, contention, flow?

Filed per Ryan 2026-05-15 01:55 UTC + 02:02 UTC ("for right now do research and designer think of how we could communicate the teams process visually... reference research and synthesizations and visual designs"). Sister to `docs/design/bpmux-advisory-signal-substrate/research.md` (#1346) and `docs/design/agent-rooms-as-homes-architecture/research.md` patterns. Pre-decomposition deliverable.

**Authoring**: designer §1-§3 + §5 (motivation + design-space + cross-tool sister-pattern + visual-design proposals); PM §4 + §6 + §7 (decomposition + cost + resurfacing-conditions); cross-role §8 (recommendation).

**Required pre-reading** (Ryan-named references):
- **The Phoenix Project** (Gene Kim) — DevOps Three Ways + Theory of Constraints applied to IT + four-types-of-work taxonomy
- **Accelerate** (Forsgren / Humble / Kim) — DORA metrics + flow-metrics-predict-performance research
- **Lean Software Development** (Mary + Tom Poppendieck) — 7 wastes + value-stream-mapping + pull-systems

Team-reading-substrate cost: ~16 hours team-time across 3 books. Implementation defers per `feedback_yagni_deferral_names_resurfacing_condition` until reading-substrate met.

---

## §1 Motivation (designer-authored)

### 1.1 Empirical trigger — Pegasus retro convergence

Pegasus iteration 2026-05-14: 13/15 stories rejected. Today's retro surfaced multiple class-of-pain that the team felt acutely but could NOT visualize in bpmux today:

- **Scope-bloat** (Pegasus 15+ stories committed; >50% reject rate) — visible only at retro-time, after the iteration burned
- **Bottleneck at desk-check** — multiple stories piled up in `delivered` state waiting for designer review; visible only by counting badges
- **Silent-role accumulation** — PM hit `api_error` repeatedly; team didn't see role-blocked state in real-time
- **Recidivism class** (4-day inline-`//` discipline-gap across 4 stories) — pattern visible only via retro-analysis, NOT during iteration
- **Filing-recidivism** (4 substrate-foundation Bugs filed in 7 minutes during today's retro; all retired 1 hour later) — pattern visible only after-the-fact

The team has the substrate-events (state-transitions, messages, timestamps, role-presence) to OBSERVE these patterns. We don't have the visual-communication substrate to SURFACE them.

### 1.2 Ryan-canon reframe (2026-05-15 01:55 UTC)

After today's morning's advisory-signal research-spike (#1346) framed the question as "what dangers to signal" + "how to signal without prescribing process," Ryan reframed:

> "what if we start with being able to map a teams process. from story inception to delivery. we could point out bottle knecks. points of contention. resouces to take into consideration would be the pheonix project, accelerate and lean books."

This is a sharper substrate-shape: bpmux maps WHATEVER process a team uses + surfaces bottlenecks + contention. Doesn't prescribe HOW to work; observes WHAT'S HAPPENING in the team's flow.

Per Phoenix Project's First Way: make work VISIBLE (left-to-right; inception → delivery). Per Lean: visualize the value-stream so waste becomes obvious. Per Accelerate: measure flow, predict performance.

### 1.3 Substrate-axis distinction (preserves #1346 boundary-respect)

The value-stream observability frame ALIGNS with #1346 research-doc §6.4 boundary-respect discipline:

- **Value-stream-mapping is bpmux-domain** — bpmux already observes story-state-transitions + message-flow + role-presence + iteration-ceremonies. Visualizing these is showing the team THEIR OWN work-shape, not prescribing how to work.
- **Bottleneck-detection is bpmux-domain** — bottlenecks are observable from coordination-state (where stories pile up; which role is overloaded; which stage has highest cycle-time)
- **Process-prescription is OUT OF SCOPE** — bpmux must NOT tell teams "your bottleneck means you should add more designers" or "you should split stories smaller." That's team-internal-decision per Ryan-canon.

The visual-substrate surfaces FACTS about the flow; teams interpret + act.

### 1.4 Cross-iteration-arc context

Today's empirical events leading to this research:
- Pegasus retro (2026-05-14 22:35 UTC): substrate-enforce > discipline-only convergence
- 4-Bug substrate-foundation overreach (filed 22:38 + retired 01:35 UTC) — first attempt at substrate-enforcement got wrong axis
- #1346 advisory-signal research-doc (01:42 UTC) — narrowed the question; established 3-class A/B/C stratification + §6.4 boundary-heuristic
- Value-stream reframe (01:55 UTC) — broader frame; advisory-signals become bottleneck-indicators within flow-map

This research-doc builds on #1346 + extends to flow-visualization.

### 1.5 Phoenix / Accelerate / Lean canonical concepts (informal — pending team reading)

**Phoenix Project (Kim)**:
- **Three Ways**: (I) flow left-to-right; (II) amplify feedback loops from right; (III) culture of experimentation + learning
- **Four-types-of-work**: business-projects / internal-projects / changes / unplanned-work — taxonomy for prioritization
- **Theory of Constraints (Goldratt; underlies Phoenix)**: identify bottleneck → exploit → subordinate everything → elevate → repeat
- **WIP-limits**: too much in-flight work degrades throughput; limit per-stage

**Accelerate (Forsgren / Humble / Kim)**:
- **DORA metrics**: lead time (inception → production) / deploy frequency / MTTR / change-failure-rate
- **Flow metrics over output metrics**: cycle-time + throughput + WIP predict performance; lines-of-code + tickets-closed do not
- **Performance bands**: elite / high / medium / low — measured by DORA combo, not single-metric
- **Capability research**: 24 capabilities including continuous-delivery + lean-management + Westrum-org-culture

**Lean Software Development (Poppendieck)**:
- **7 wastes**: partially-done-work / extra-features / relearning / handoffs / task-switching / delays / defects
- **Value-stream-mapping (VSM)**: physical map of work-stages + cycle-time + wait-time per stage + handoff-points; sister to factory floor-plan
- **Pull-systems (Kanban)**: downstream pulls work when ready; prevents push-overload
- **Eliminate-waste at source**: fix the process that creates defects, not just the defects

### 1.6 Open architectural question (this research-spike's scope)

GIVEN bpmux observes coordination-state already + GIVEN Lean/Phoenix/Accelerate canon provides the value-stream visualization vocabulary, what SHOULD bpmux's visual-substrate render?

This research surveys: (§2) what process-axes to visualize; (§3) cross-tool visual sister-pattern; (§5) concrete visual-design affordance proposals. Implementation defers per resurfacing-conditions.

---

## §2 Design-space — what to visualize (designer-authored)

### 2.1 The team's value-stream — bpmux's observable substrate

Today bpmux observes (canonical-store + message-log + iteration-ceremony):

**Story-lifecycle stages** (observable from state-transition history):
- Filed (story-creation timestamp)
- Iceboxed (pre-commit-to-iteration)
- Ready (committed to iteration; awaiting pickup)
- In-progress (dev picked up; impl-in-flight)
- Desk-check (delivered; awaiting designer review)
- Delivered (passed desk-check; awaiting accept)
- Accepted (closed-with-accept)
- Retired (closed-without-accept)

Plus rework loops (rejected back to ready; fail_desk_check back to in-progress).

**Per-stage cycle-time data** (observable from transition timestamps):
- Time from filed → iceboxed (PM filing-discipline latency)
- Time from iceboxed → ready (ratification-ceremony cadence)
- Time from ready → in-progress (pickup-latency; signals dev-capacity)
- Time from in-progress → desk-check (impl-cycle-time)
- Time from desk-check → delivered (designer-review-latency; signals designer-capacity)
- Time from delivered → accepted (owner-walk-latency)

**Role-presence + message-flow** (observable from message-log):
- Last-broadcast timestamp per role
- Message-volume per role per hour
- Send_message failures (api_error class)
- Permission-prompt-class messages

**Iteration-ceremony** (observable from iteration-state):
- Iteration scope-size (story-count committed)
- Iteration ship-rate (accepted / committed)
- Iteration cycle-time (open → close)

### 2.2 Process-axes worth visualizing per Lean / Phoenix / Accelerate

**A. Flow-visualization (Lean VSM + Phoenix First Way)**:
- Left-to-right linear-flow visualization (inception → delivery)
- Each stage = node; each transition = edge
- Per-node: stories-currently-in-state + average-cycle-time + last-touch
- Per-edge: transition-frequency + reverse-edges (reject_story / fail_desk_check = reverse-flow)
- Surfaces: which stage is BUSIEST (most in-flight) + which stage has LONGEST cycle (potential bottleneck)

**B. Bottleneck-indicators (Theory of Constraints; Phoenix Second Way)**:
- Identify the stage with highest WIP-or-cycle-time
- Mark visually (color-shift / pulse / size-emphasis)
- Show "subordinate to bottleneck" suggestion class (passive-signal per #1346 §6.3)
- Avoid prescribing: bpmux shows WHERE the bottleneck IS; team decides whether to add capacity / split work / wait

**C. WIP-counts per stage**:
- Per-column count badge (sister to existing column-bucketing render)
- Color-shift when WIP exceeds team-historical-mean (advisory; not block)
- Empirical-team WIP-limit auto-learned from rolling-window observation

**D. Flow-efficiency (Lean: touch-time / cycle-time ratio)**:
- For accepted stories: time-in-active-stages (in-progress + desk-check + delivered) / total-cycle-time
- Typical software-team flow-efficiency = 15-25% (most time = wait, not work)
- Visualize as ratio per story OR per iteration average

**E. DORA metrics dashboard (Accelerate)**:
- Lead time (story-filed → accepted; not yet "production" since bpmux doesn't observe deploy)
- Deploy frequency proxy (accepted-stories-per-day; rolling average)
- MTTR proxy (rework-cycle-time: reject_story → re-deliver → re-accept)
- Change-failure-rate (reject_story-count / total-accept-count)

**F. Iteration-burnup / burndown** (Pivotal-Tracker + Jira sister):
- Stories accepted-over-time within iteration
- Velocity-trend across iterations
- Iteration-ship-rate (accepted / committed) — sister to Pegasus's 2/15 empirical

**G. Four-types-of-work pie (Phoenix Project taxonomy)**:
- Business (Feature) / Internal-Projects (Chore) / Changes (Bug-fix) / Unplanned-Work (interrupts; rework cycles)
- Visualize ratio per iteration; high unplanned-work signals process-stability issue

**H. Role-load visualization**:
- Stories-in-flight per role
- Time-since-last-broadcast per role (presence-indicator)
- Handoff-edges between roles (PM → dev → designer → owner)
- Surfaces: role-overload (lots-of-stories) + role-silence (no-broadcast for X hours)

### 2.3 What's OUT of scope per Ryan-canon (preserves #1346 §6.4 boundary)

- Recommending team-actions ("add more designers" / "split this story" / "stop committing to iteration")
- Comparing team to other teams (cross-team-comparison-bias risk)
- Prescribing iteration-cadence or velocity-target
- Lint-level rules (no-inline-comments / falsifier-fidelity / etc; these stay in #1345 layer)
- Project-management-prescription (Scrum / Kanban / SAFe / etc; bpmux maps the team's CURRENT process, doesn't prescribe a methodology)

The visual-substrate SHOWS the team THEIR OWN flow; team interprets + acts.

---

## §3 Cross-tool visual sister-pattern (designer-authored)

### 3.1 Linear

- **Flow visualization**: kanban-board left-to-right per workflow-state; per-state count-badge
- **Cycle-time**: per-state cycle-time displayed on hover; rolling-average exposed in analytics
- **DORA**: "Triage / In-Progress / Done" defaults; cycle-time + throughput in analytics-tab
- **Bottleneck**: implicit via WIP-count; no explicit "this is the bottleneck" marker
- **Sister-applicability for bpmux**: column-bucketing already exists; add per-column WIP + cycle-time annotations + analytics-tab as separate surface

### 3.2 Jira

- **Flow visualization**: per-project Kanban / Scrum boards; configurable swimlanes
- **Cycle-time**: via Jira's Cycle Time Report (control-chart visualization)
- **DORA**: opt-in via add-ons; not first-class
- **Bottleneck**: via Cumulative Flow Diagram (CFD) — band-width per state shows WIP-buildup
- **Sister-applicability for bpmux**: CFD-pattern is canonical visual for bottleneck-identification; sister to §5.3 proposal below

### 3.3 GitHub Projects

- **Flow visualization**: customizable views (board / table / roadmap); per-column count
- **Cycle-time**: Insights tab with iteration-level metrics
- **DORA**: not first-class; community-built via GraphQL queries
- **Bottleneck**: passive via column-WIP visibility
- **Sister-applicability for bpmux**: clean column-view + dedicated Insights-tab pattern; sister to bpmux's existing board + proposed analytics-panel

### 3.4 Pivotal Tracker (legacy reference)

- **Flow visualization**: vertical iteration-grouping + velocity-projection chart
- **Cycle-time**: per-iteration acceptance velocity
- **DORA**: velocity-based projections (proto-DORA)
- **Bottleneck**: implicit via iteration-spillover indicators
- **Sister-applicability for bpmux**: velocity-projection-chart is sister to Iteration-Cassiopeia + future Iteration-Andromeda planning

### 3.5 Trello

- **Flow visualization**: classic kanban-board; per-list count via power-ups
- **Cycle-time**: power-up-dependent
- **DORA**: power-up-dependent
- **Bottleneck**: implicit; no first-class
- **Sister-applicability for bpmux**: minimalist board-view is starting-point sister; bpmux has already passed Trello's substrate-depth

### 3.6 Kanban-tools (LeanKit / Kanbanize)

- **Flow visualization**: kanban-board WITH explicit WIP-limits per column (Kanban-method-canonical)
- **Cycle-time**: per-card cycle-time visible + rolling-average per stage
- **DORA**: cycle-time-distribution histogram + scatter-plot per card-class
- **Bottleneck**: CFD + WIP-limit-violation visual (column-red-when-overflow)
- **Sister-applicability for bpmux**: WIP-limit-visualization is the IDIOMATIC pattern; sister to §5.3 + §5.4 proposals

### 3.7 Plandek / LinearB / Swarmia (DORA-focused tools)

- **Flow visualization**: DORA dashboards + cycle-time-distribution + handoff-latency analysis
- **Cycle-time**: deep-stage breakdown (PR-cycle-time / review-time / merge-time / deploy-time)
- **DORA**: first-class — all 4 DORA metrics + trend over time
- **Bottleneck**: stage-with-highest-latency callout
- **Sister-applicability for bpmux**: DORA-as-first-class is the FORWARD-LEAN pattern; sister to §5.5 + §5.6 proposals

### 3.8 Cross-tool convergence

**Pattern**: industry-default has 3 visual-substrate layers:
1. **Board-level WIP visibility** (Linear / Jira / GitHub Projects / Trello / Kanbanize) — column-bucketing + count
2. **Analytics tab** for cycle-time + cumulative-flow + DORA (Jira / Linear / Kanbanize / GitHub Projects)
3. **Specialized DORA dashboards** (Plandek / LinearB / Swarmia) — separate-product class

**Implication for bpmux**:
- Layer-1 already exists (board.tsx column-view); ENHANCE with per-column WIP-count + cycle-time annotations
- Layer-2 is NEW for bpmux; add analytics-panel (sister to Epic #1236 Velocity + DORA charts panel)
- Layer-3 is over-scope; defer per resurfacing-condition

### 3.9 Visual-design principle convergence

Across all tools surveyed, common design principles:

- **Spatial-mapping = process-flow**: left-to-right (Kanban) OR top-to-bottom (Linear vertical-stack) reveals direction-of-progress
- **Count-badges** = WIP per stage; size correlates with attention-deserve
- **Color-shift** = signal-class (in-progress green / blocked red / stale yellow)
- **Hover-detail** = per-item context without cluttering the overview
- **Drill-in** = navigate from overview → per-item-detail
- **Time-axis charts** = trend over time (CFD / burnup / DORA-rolling-window)

bpmux already implements 4 of 6 (spatial-mapping ✓ count-badges ✓ hover-detail ✓ drill-in ✓). NEW patterns for value-stream-observability: color-shift-on-stage-stall + time-axis charts.

---

## §4 Decomposition (PM-authored — pending)

*PM authors per §1.5 stakeholder-alignment + research-doc lessons. Anticipated shape:*
- *Decomposition principle: research-spike → team-converse → file follow-up post-converge*
- *Three-layer-stratification per §3.8 (board-WIP-enhancement + analytics-panel + deferred DORA-dashboards)*
- *Layer-1 cost (extend existing column-render)*
- *Layer-2 cost (new analytics-panel substrate)*
- *Iteration-budget across 2-3 iterations if all layers ship*

---

## §5 Visual-design proposals (designer-authored)

### 5.1 Enhanced board column-view (Layer 1)

**Affordance shape**: per-column header annotation
- WIP-count badge (already exists via story-count)
- Average cycle-time per stage (NEW): rolling-30-day mean displayed under column-name
- Color-shift when WIP > historical-mean (NEW): passive-visual signal

**Mock layout**:
```
┌─ READY (3) ─────┐  ┌─ IN-PROGRESS (5) ──┐  ┌─ DESK-CHECK (2) ────┐  ┌─ DELIVERED (1) ──┐
│ avg 2.1d        │  │ avg 4.3d  ⚠HIGH    │  │ avg 0.6d            │  │ avg 1.4d         │
├─────────────────┤  ├────────────────────┤  ├─────────────────────┤  ├──────────────────┤
│ #1297 Slice A   │  │ #1303 RTL infra    │  │ #1345 disc-gates    │  │ ...              │
│ #1300 Slice C   │  │ #1304 RTL archt    │  │ #1339 Slice B drag  │  │                  │
│ ...             │  │ ...                │  │                     │  │                  │
└─────────────────┘  └────────────────────┘  └─────────────────────┘  └──────────────────┘
```

The `⚠HIGH` indicator surfaces bottleneck PASSIVELY (Linear / Kanbanize sister). No block; user-aware.

**Cost**: low (extend existing column-render component); UX-cost low.

### 5.2 Per-story cycle-time annotation

**Affordance shape**: hover-tooltip OR inline-badge on story-row
- Time-in-current-state (e.g., "in-progress 3.2 days")
- Highlighted if stale-class (>2σ above team-average)

**Mock**:
```
○ #1300 [Feature] close_iteration positional        in-progress 4.1d ⚠
○ #1339 [Feature] Drag iteration marker             in-progress 0.5d
```

The `⚠` indicates stale-class; designer-row-class (per-row badge) sister to existing rework_comment ↩ glyph.

**Cost**: low (extend StoryRow render); UX-cost low.

### 5.3 Cumulative Flow Diagram (CFD) panel (Layer 2)

**Affordance shape**: dedicated panel below board OR separate tab
- X-axis: time (last 30 days)
- Y-axis: count of stories
- Stacked-area chart: each band = one stage (icebox / ready / in-progress / desk-check / delivered / accepted)
- Band-width WIDENING = WIP-buildup in that stage (bottleneck-indicator)
- Band-narrowing = healthy throughput

**Sister-pattern**: Jira's CFD report; canonical Kanban visualization.

**Mock-shape**:
```
30 ─┐
    │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ ICEBOX (light)
    │██████████████████████████████ ACCEPTED (dark)
20 ─┤▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ DELIVERED (mid)
    │▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ DESK-CHECK (mid-light)
    │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ IN-PROGRESS (light)
10 ─┤
    │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ READY (mid)
 0 ─┴──────────────────────────────────────────
     -30d                              today
```

When a band WIDENS over time, that stage is accumulating WIP = bottleneck. Team reads + acts (passive-signal per #1346 §6.3).

**Cost**: medium (new chart-substrate + canonical-store time-series query); UX-cost medium.

### 5.4 DORA metrics dashboard (Layer 2)

**Affordance shape**: 4-tile dashboard sister to Plandek / LinearB pattern

**Mock**:
```
┌────────────────────────────────┐  ┌────────────────────────────────┐
│ LEAD TIME                      │  │ DEPLOY FREQUENCY               │
│   median: 5.2d                 │  │   accepted/day: 0.8            │
│   p90:    12d                  │  │   trend: ↗ +15%                │
│   trend:  ↘ -8%                │  │                                │
└────────────────────────────────┘  └────────────────────────────────┘

┌────────────────────────────────┐  ┌────────────────────────────────┐
│ MTTR (rework cycle)            │  │ CHANGE FAILURE RATE            │
│   median: 1.4d                 │  │   reject-rate: 13%             │
│   trend:  → flat               │  │   trend: ↗ +5% ⚠               │
└────────────────────────────────┘  └────────────────────────────────┘
```

Per Accelerate: elite-performers have lead-time <1day + deploy-frequency >1/day + MTTR <1hr + change-failure-rate <15%. bpmux doesn't observe production-deploy; uses ACCEPTED as deploy-proxy.

**Cost**: medium (new dashboard substrate + DORA-computation backend); UX-cost medium.

### 5.5 Role-load + presence indicator

**Affordance shape**: per-role badge in roster panel
- Stories-in-flight count (in-progress per role; desk-check per role; etc)
- Time-since-last-broadcast (presence-state ambient/elevated/critical per #1325 3-state UX)
- Handoff-edges visual (PM → dev → designer → owner)

**Mock**:
```
┌─ Team ─────────────────────────────────────────┐
│ 🟢 pm        2 stories in-flight  · 5min ago  │
│ 🟢 dev       3 stories in-flight  · 1min ago  │
│ 🟡 designer  4 stories in-flight  · 12min ago │  ← overload signal
│ ⚪ owner     waiting on accept      · 1h ago   │
└────────────────────────────────────────────────┘
```

Surfaces role-overload + role-silence patterns. Sister to today's empirical: designer was overloaded on desk-check at Pegasus; visible only retroactively.

**Cost**: low-medium (per-role aggregation substrate); UX-cost low.

### 5.6 Iteration burnup chart

**Affordance shape**: per-iteration progress visualization
- X-axis: time (iteration open → close)
- Y-axis: cumulative accepted-count
- Ideal-line: linear from 0 to committed-count
- Actual-line: actual cumulative accepts

**Mock**:
```
14 ─┐                                            committed
    │
12 ─┤                                  /-- ideal (linear)
    │                              ___/
10 ─┤                          ___/
    │                      ___/
 8 ─┤                  ___/                           actual
    │              ___/
 6 ─┤          ___/______ ← stalled here (bottleneck)
    │      ___/
 4 ─┤  ___/
    │_/
 2 ─┤
 0 ─┴────────────────────────────────────────────
    day 1    day 2    day 3    day 4    day 5
```

Surfaces iteration-stall in real-time. Today's Pegasus 2/15 ship-rate would have shown this curve diverging by day-2.

**Cost**: low-medium (per-iteration accept-history aggregation); UX-cost low.

### 5.7 Affordance-spectrum summary

Per #1346 §3.9 affordance-shape framework, this research's visual-substrate proposals:

| Affordance | Class | Cost | First-Layer? |
|---|---|---|---|
| §5.1 Enhanced column-view (WIP + cycle-time) | Inline-badge + tooltip | Low | YES (Layer 1) |
| §5.2 Per-story cycle-time annotation | Inline-badge | Low | YES (Layer 1) |
| §5.5 Role-load indicator | Side-panel | Low-medium | YES (Layer 1) |
| §5.6 Iteration burnup chart | Time-axis chart | Low-medium | YES (Layer 1; sister to existing iteration substrate) |
| §5.3 CFD panel | Time-axis chart panel | Medium | Layer 2 |
| §5.4 DORA dashboard | Multi-tile dashboard | Medium | Layer 2 |

**Sequencing recommendation**: Layer-1 affordances (§5.1, §5.2, §5.5, §5.6) ship first as smallest-shippable per `feedback_smallest_first_commit`. Layer-2 affordances (§5.3, §5.4) defer until Layer-1 lands + team-converse on whether they add value.

### 5.8 A11y substrate (designer-axis)

Per existing bpmux a11y discipline (sister to today's a11y-audit skill):
- All chart visualizations have ARIA-label describing the data-shape
- Per-stage WIP-counts + cycle-times announced via `aria-live` regions when changed
- Color-shift signals MUST pair with glyph (color-blind-safe)
- Keyboard-navigable: charts have alternate data-table view (`role="table"` fallback)
- Sister to designer's standing a11y substrate for renderer-class stories

---

## §6 Cost/complexity (PM-authored — pending)

*PM authors per §5.7 sequencing recommendation. Anticipated shape:*
- *Per-affordance impl-cost in stories*
- *Cross-affordance shared-substrate (canonical-store time-series query + chart-component library)*
- *Iteration-budget across 2-3 iterations if all layers ship*

---

## §7 Resurfacing-conditions for follow-up filings (PM-authored — pending)

*PM authors per `feedback_yagni_deferral_names_resurfacing_condition`. Anticipated triggers:*
- *Empirical convergence on Layer-1 affordance value*
- *Team-pre-reading completed*
- *Sister-substrate (#1236 Epic) ratified for Layer-2 home*

---

## §8 Owner-recommendation (cross-role draft — pending)

*Pending §4-§7 PM authoring + cross-role converge + Ryan-ratify.*

---

## Cross-references

- `#1346` advisory-signal research-doc — sister research-doc; 3-class A/B/C boundary discipline applies
- `Epic #1236` Velocity + DORA charts panel (iceboxed) — likely implementation-Epic for Layer-2 affordances
- `docs/design/agent-rooms-as-homes-architecture/research.md` — sister co-authoring pattern
- Ryan-canon 2026-05-15 01:55 UTC value-stream reframe
- Ryan-canon 2026-05-15 02:02 UTC "do research and designer think... reference research and synthesizations and visual designs"
- Phoenix Project / Accelerate / Lean canonical references (pre-reading pre-requisite)
- `feedback_yagni_deferral_names_resurfacing_condition` — defer-with-named-resurfacing-condition
- `feedback_bpmux_is_generic_team_coordination_not_team_process` — boundary-respect (preserved per §2.3)
- Pegasus retro 2026-05-14 — empirical motivation per §1.1

---

## Authoring status

- §1 Motivation: ✓ designer (2026-05-15 02:05 UTC)
- §2 Design-space: ✓ designer (2026-05-15 02:05 UTC)
- §3 Cross-tool sister-pattern: ✓ designer (2026-05-15 02:05 UTC)
- §4 Decomposition: pending PM
- §5 Visual-design proposals: ✓ designer (2026-05-15 02:05 UTC)
- §6 Cost/complexity: pending PM
- §7 Resurfacing-conditions: pending PM
- §8 Owner-recommendation: pending cross-role converge

Open questions for team-converge:
- (i) Does the visual-spatial-mapping per §3 cross-tool convergence (column-WIP / CFD / DORA-dashboard / role-load) align with Ryan's "communicate the team's process visually" intent?
- (ii) Sequencing per §5.7 — does Layer-1 first / Layer-2 deferred match owner-priority?
- (iii) #1236 Epic-rescope to "value-stream observability" — preferred, or keep narrow "DORA charts panel"?
- (iv) Pre-reading pre-requisite — team commits to reading Phoenix + Accelerate + Lean BEFORE implementation filings? Or implementation grounds in research-doc + on-demand reading?
