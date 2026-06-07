# Velocity + DORA charts — chart-type research

*Designer research-spike per Ryan 2026-05-09 15:37 UTC. Builds on `docs/research/foundations/2026-04-06-accelerate-dora-for-bpmux.md` (DORA-for-bpmux mapping) + `applications/electron/src/renderer/src/metrics/Metrics.tsx` (current numerical panel, already shipped). This research is the input for the bigger charts Feature (gated on this research-spec).*

**Designer apply-gap acknowledged**: initial draft assumed the numerical panel was unbuilt. The panel ships at `Metrics.tsx` as a `<dialog>` with 5 metric tiles + inline `<Bar>` SVG indicators (per Ryan screenshot 2026-05-09 15:40 UTC + #1233 retired-as-filing-error 15:41 UTC). This research now grounds in that substrate — what charts ADD beyond numbers + bar-against-current-max indicators.

## Current substrate (already shipped)

`Metrics.tsx`:
- 5 metric tiles in a `<dl>` grid: Delivered/day (14d), Delivered in window, Median pickup→deliver, Median deliver→accept, Rework rate
- Each tile = label + numerical value + (optional) inline `<Bar>` SVG showing current value vs a hard-coded max (e.g., delivered/day max 5, with `--accent-success` green fill)
- Theme-aware via CSS-var color tokens (`var(--accent-success)`, `var(--accent-warning)`, etc.)
- Aria-labeled; closeable via Escape; non-modal `<dialog>.show()` per native primitive

The current substrate answers: *what is the current value? how does it compare to a static target?* That's the at-a-glance user-job for "is the project healthy right now?"

## What charts add beyond numbers + bars

The numerical panel + bar-against-max can't answer:

1. **Trend over time** — Is delivered/day climbing or degrading? A bar against a fixed max only shows current-vs-target; it can't show whether the value is in a one-off spike or a sustained trend. A sparkline next to the number reveals direction + volatility.
2. **Distribution shape** — Median pickup→deliver = 27m hides whether MOST stories complete in 5m with one outlier at 4h, or every story takes ~25-30m consistently. Two distributions with the same median have very different operational implications. A box-and-whisker or per-story dot-plot shows the shape.
3. **Cross-metric correlation** — When delivered/day drops, does rework rate spike? When Median deliver→accept widens, does delivered-in-window contract? Side-by-side trend-lines reveal coupling that numerical tiles can't surface.

Three user-jobs charts add. If a metric only ever needs current-value + target-comparison, leave it as a bar-tile in the existing panel. Promote to chart only when trend, distribution, or correlation is the user-job.

1. **Trend visibility** — Is throughput improving or degrading? A single number can't show direction without a delta-arrow that hides the noise. A line over time shows direction *and* volatility.
2. **Distribution shape** — Cycle time mean = 3.2h hides whether most stories complete in 1h with one outlier at 24h, or every story takes ~3h consistently. Distribution shape matters for capacity planning.
3. **Cross-metric correlation** — When deployment frequency drops, did rejection rate spike? Side-by-side trend lines reveal coupled-failure patterns numerical text can't.

If a metric only ever needs the *current value*, leave it in the numerical panel. Promote to chart only when trend, distribution, or correlation is the user-job.

## Per-metric chart-type recommendation

Mapping each of the 6 signals from the bpmux DORA research to a chart shape:

| Metric | bpmux Mapping | Recommended Chart | Rationale |
|---|---|---|---|
| Deployment Frequency | Stories accepted per session | **Bar chart** (per session, last N) | Discrete count; bars show binary "shipped this session vs not"; gaps in the timeline are themselves diagnostic |
| Lead Time | Story start → acceptance | **Box-and-whisker per week** OR **scatter dot-plot per story** | Distribution shape matters; median + IQR + outliers > mean line; per-story dot reveals individual outliers without aggregating away |
| Change Failure Rate | Stories rejected at acceptance / total presented | **Stacked bar per week** (accepted + rejected) | Two-color stack shows ratio AND absolute counts; a single percentage line loses the absolute-count signal |
| MTTR | Rejection → re-acceptance | **Histogram** OR **CDF** (cumulative distribution) | Skew shows whether most rejections recover fast with a long tail vs uniformly slow; CDF answers "what % of rejections recover within X hours?" |
| WIP count | Stories in progress now | **Sparkline** (last N hours) | Live signal; sparkline next to the live number shows whether WIP is climbing (review-bottleneck warning) or steady |
| Rework ratio | Delivery cycles before acceptance (1 = clean, 2+ = rework) | **Stacked bar per week** (1-cycle / 2-cycle / 3+-cycle) | Categorical distribution; stacking reveals shift toward higher-rework over time |

### Why not other shapes

- **Pie chart**: rejected for distribution-of-categorical because absolute totals matter; stacked bar wins.
- **Heatmap**: appealing for time-of-day patterns but sample-size at a 1-human-3-agent team is too small to see day-of-week signal reliably; defer until Phase-2+ when there's enough history.
- **Cumulative-flow diagram**: industry-standard for kanban WIP across columns over time. Strong fit for bpmux's 6-state board (Icebox/Ready/InProgress/DeskCheck/Delivered/Accepted) but expensive to render correctly + needs data-density we may not have yet. Defer to Phase-2 charts story.
- **Gauge / dial**: rejected for current-value display per WCAG + a11y reasoning — gauges are visually striking but hide ratio comparisons and don't translate to keyboard/screen-reader well.

## Display option taxonomy

Across the 6 metric/chart pairings, the rendering primitives needed:

1. **Bar (vertical, single)** — Deployment Frequency
2. **Bar (vertical, stacked, 2-color)** — Change Failure Rate
3. **Bar (vertical, stacked, N-color)** — Rework Ratio
4. **Box-and-whisker** — Lead Time (option 1)
5. **Scatter / dot-plot** — Lead Time (option 2)
6. **Histogram (frequency bars)** — MTTR (option 1)
7. **CDF (line, monotonic)** — MTTR (option 2)
8. **Sparkline (line, inline-with-text)** — WIP

Total of ~5 distinct chart-component types if implemented as a small library. Recharts / Visx / D3 ecosystems all cover these. Hand-rolled SVG is also viable at this scope (each chart is ~50-100 LoC of pure SVG-rendering — see `applications/electron/src/renderer/src/ci/CiGlyph.tsx` for the precedent of inline-SVG rendering against tokens-as-CSS-vars).

## Substrate constraints

### Dark theme + WCAG AA contrast

Per today's #103 + #425 audit substrate, the chart palette must:
- Pass WCAG AA contrast (4.5:1) for chart-axis text against `--bg-primary`
- Pass 3:1 for non-text components (chart strokes, fills) against the same
- Use the existing token palette from `--fg-primary` / `--fg-secondary` / `--fg-tertiary` rather than introducing a new chart-color set
- For multi-series charts (stacked bars), reuse the role-color tokens from `roleColor.ts` (dynamic HSL hue, deterministic per-series)

### Theme-aware

`applications/electron/src/renderer/src/lib/useTheme.ts` resolves `data-theme` on `<html>`. Charts must read CSS-vars (not hard-coded colors) so dark/light theme switches without re-render. Pattern: `<rect fill="var(--accent-success)" />` not `<rect fill="#22c55e" />`. Sister to today's #103's contrast audit logic.

### Family-pattern fit with existing surfaces

Existing TUI/Electron surfaces are text-rendered (board columns, sidebar, drill-in, comment list). Charts are a NEW visual-substrate axis but should fit the family by:
- Reusing existing CSS-grid layout primitives for chart placement (sister to `<Board>` grid layout)
- Sharing the "card" surface treatment (`--surface-elevated` background, `--border-subtle` outline, ~8px padding)
- Hover-tooltip for per-data-point detail (sister to existing tooltip patterns in StoryBadge / CiGlyph)
- Keyboard-traversable per `feedback_visual_class_falsifier_needs_visual_assertion` extended — focus-visible on chart elements when they carry interactive affordance (e.g., click-to-drill-in on a bar)

### A11y substrate (per #425's 4-axis audit)

Charts must:
- Carry `role="img"` + `aria-label` describing the chart's user-job ("Deployment frequency, last 14 days, 8 stories accepted")
- Provide a `<table>` data alternative behind a "Show data" affordance for screen-reader users
- Avoid color-as-only-channel for stacked-series differentiation — use pattern/hatch alternatives or labeled legend

### Substrate constraints summary

The chart Feature should NOT add a heavy chart library (Recharts/Visx ~100KB+). Hand-rolled SVG with CSS-var color-tokens fits the substrate; sister-precedent at CiGlyph.tsx. New dependencies trigger the YAGNI-on-substrate trap per `feedback_yagni_substrate_trap`.

## Family-pattern fit with existing visual surfaces

| Surface | Visual pattern | Charts fit |
|---|---|---|
| Board columns | CSS-grid + collapsible per column | Charts as a NEW collapsible section above board, OR per-column sparkline |
| StoryBadge | Inline glyph + tooltip | Sparkline-as-glyph for per-story trend |
| CiGlyph | SVG with CSS-var color | Direct precedent for chart SVG pattern |
| Sidebar groups | Hierarchical text list | Numerical panel (#1233) sister; charts adjacent |
| Drill-in | Long-form vertical scroll | Charts could embed inline at top of drill-in for per-story metrics |

The most natural placement: a "metrics" section ABOVE the board (sister to project-view/reshape.md's numerical panel, but with chart visualization). Single-screen-reach so Ryan can glance + know.

Alternative: a separate `/metrics` slash-command surface. This pulls charts out of the daily flow, which loses the at-a-glance value. Reject in favor of board-adjacent placement.

## Implementation sequencing recommendation

If charts ship as one Epic with multi-slice children:

1. **Slice 1 — Sparkline primitive** + WIP sparkline integration. Smallest first; locks the SVG + CSS-var + a11y substrate without trying to do everything at once.
2. **Slice 2 — Bar chart primitive** + Deployment Frequency + Change Failure Rate (uses the same primitive with single-color and stacked-color variants).
3. **Slice 3 — Box-and-whisker OR Scatter** for Lead Time (designer-call at slice-time which display shape ratchets best).
4. **Slice 4 — Histogram or CDF** for MTTR (sister-call to slice 3).
5. **Slice 5 — Rework Ratio stacked bar** (replicates slice 2 pattern).

Each slice is a pure-rendering component with at most 1 new chart-shape; canonical-archetype validates first per `feedback_canonical_archetype_pre_pickup_review`.

## Out of scope for this research

- **Real-time chart updates** — Charts re-render when underlying metrics change; live-streaming is sister-substrate to WS-only state flow (Epic #1208). Defer.
- **Date-range picker** — "last 14 days" is a static window for v1. Date-range picker UI is sister-Feature.
- **Drill-down click-to-filter** — Click on a bar shouldn't filter the board in v1; defer to a dedicated interaction Feature once charts stabilize.
- **Export chart as PNG/SVG** — Useful for stakeholder reporting; not needed for the at-a-glance user-job.

## References

- Forsgren, Humble, Kim. *Accelerate: The Science of Lean Software and DevOps* (2018). DORA metrics canon. Already mapped to bpmux at `docs/research/foundations/2026-04-06-accelerate-dora-for-bpmux.md`.
- Henrik Kniberg. Cumulative-flow-diagram canon for Kanban WIP-over-time.
- Edward Tufte. *Visual Display of Quantitative Information* — sparkline primitive originator; small-multiples principle.
- Stephen Few. *Information Dashboard Design* — guides the "what to put on the at-a-glance surface" question; reject pie + gauge in favor of bar + sparkline.
- WCAG 2.1 SC 1.4.11 (non-text contrast); 1.1.1 (text alternatives) — chart a11y baseline.
- bpmux precedent: `applications/electron/src/renderer/src/ci/CiGlyph.tsx` — inline SVG + CSS-var colors + theme-aware rendering.
- bpmux precedent: `applications/electron/tests/playwright/a11y-audit.spec.ts` (today's #425 ship) — measurable a11y verification at desk-check.

## Three Feature-shape options

Refined per "what charts ADD over current substrate" framing:

### Option A — Augment existing tiles with sparklines
Each Metrics.tsx tile gets a sparkline next to the bar showing the metric's last 14 days/sessions. Smallest-change; tile shape preserved; sparkline as a secondary visual channel under the existing bar. Lowest cost; lowest visual disruption. Doesn't address distribution-shape or correlation user-jobs.

### Option B — Replace bars with trend chart per metric
Each tile drops the current `<Bar>` (current-vs-max) and instead shows a chart-shape appropriate to that metric (line trend / distribution / etc per the per-metric table below). Highest pedagogical-density per tile; loses the at-a-glance current-vs-max comparison.

### Option C — Add a charts panel adjacent to the numbers panel
Numbers + bars stay as-is in `Metrics.tsx`; new charts panel (separate `<dialog>` OR new project-view section) provides trend + distribution + correlation views. Most additive; preserves at-a-glance + adds drill-down. Highest implementation cost.

**Recommend Option C** as Epic-shape: each chart child-Feature ships independently; numerical panel preserved as the at-a-glance surface; charts panel added as the trend-and-distribution surface. Sister-pattern to today's #103 (numerical contrast tile) + #425 (4-axis a11y audit) staying as separate test-substrates rather than merging.

## Open questions for Epic-shape decision

When the bigger charts Feature picks up:

1. **All 5 metrics or subset?** Numerical panel covers all 5 with bars; charts could add 3 highest-leverage (Median pickup→deliver distribution + Delivered/day trend + Rework Ratio trend) and defer the rest.
2. **Charts placement** — separate `<dialog>` (sister to current Metrics.tsx) OR project-view-embedded? This research recommends separate dialog for v1 (preserves the existing dialog-affordance pattern); project-view embedded as Phase-2 if Ryan reaches for charts at-a-glance frequency.
3. **Hand-rolled SVG vs library?** Recommend hand-rolled per substrate-fit + existing Metrics.tsx `<Bar>` precedent; needs dev ratify at Epic-shape time.
4. **Per-week / per-session / per-day x-axis?** Session-based aligns with bpmux work-rhythm; week-based aligns with industry DORA reporting; defer to slice-time decision.
5. **Augment vs Replace decision per-metric** (Option A/B/C): Ryan's call which metric needs which shape. This research's per-metric table is a starting recommendation, not a directive.
