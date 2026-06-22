# Chart visual-system archetype (#1237 — smallest-first archetype for Epic #1236 chart-Features)

The velocity sparkline (#1237) is the **smallest-first archetype**: it AUTHORS the chart visual-system
tokens that every subsequent Epic #1236 chart-Feature inherits. Everything below INHERITS from the
shipped token system (`applications/electron/src/renderer/src/styles/tokens.css`) — no new
color/spacing primitives, only chart-role mappings + the few chart-specific commits (stroke-weight,
empty-state, entry-motion). Grounded firsthand against the shipped tokens + the existing `Bar`
primitive (`metrics/Metrics.tsx:35-51`) + the theme cascade (`hooks/useTheme.ts`).

## Committed chart tokens (the archetype)

**Border-radius (chart-card surfaces): `4px`** — the shipped primary (Metrics.css / CiGlyph.css).
Chart cards/tiles use 4px. No new value; commit to the ONE existing primary (per the ui-audit
"pick ONE style and commit").

**Stroke-weight (sparkline + all chart line-elements): `1.5px`.** Rationale (data-ink): 1px hairlines
vanish at sparkline scale; 2px reads heavy + muddies the trend-shape; **1.5px is the crisp-but-light
balance.** ONE stroke-weight for all chart line-elements (the ui-audit "consistent stroke weight").

**Type-scale (chart text → existing font tokens, no new scale):** the shipped scale already IS the
1.125 Major-Second ratio; charts map onto it, never fork it.
- chart axis-label / micro-text → `--font-size-12` (0.75rem)
- chart tooltip-text → `--font-size-12`
- chart title → `--font-size-14`

**8pt-grid spacing (chart-internal → existing `--space-*` tokens):**
- sparkline ↔ label gap → `--space-6` (6px, matches the metric-tile stack-gap)
- axis-label gap → `--space-4` (4px); legend-item gap → `--space-8` (8px)
- data-point horizontal spacing → derived (`chart-width ÷ (N−1)`)

**Color (semantic, theme-aware via `var(--*)` — NEVER hardcoded):**
- sparkline stroke → `var(--accent-success)` (healthy-flow green; flips light/dark via the theme cascade)
- **the SHAPE carries the trend-signal** (climbing line = review-bottleneck warning · steady = healthy);
  sparklines are shape-read, not trend-color-coded — a single semantic stroke + the line-shape IS the
  glance. (Trend-color-coding is an explicit non-goal: it doubles the encoding the shape already gives.)
- endpoint dot (current WIP) → `var(--accent-success)` filled, r≈2px — anchors the most-recent value.

**Empty-state (Scenario-4, first-class):** WIP-history with <2 points → render a placeholder
text "no data yet" (`--fg-dim`, `--font-size-12`) occupying the sparkline's box — **NOT a flat-line**
(a flat-line reads as "WIP=0 steady" = a false signal) and NOT nothing (which shifts layout). The
placeholder holds the box + is honest.

**Entry-motion (chart mount): the line DRAWS IN** — `stroke-dashoffset` animates full→0 (left-to-right
draw) over `--motion-duration-medium` (250ms) with `--motion-ease-out`. `prefers-reduced-motion` →
INSTANT (line appears complete, no draw). This is the canonical chart-mount motion (sister-pattern for
all #1236 chart-Features); **the motion + its reduced-motion fallback ship in the SAME slice** (the
motion-canon). It-plays verified via a frame-sequence trace at impl (motion-live-verification), not a
static capture.

## Sparkline component spec

- `<svg role="img" aria-label="WIP trend: {N} items, {trend-word} over {window}">` — the aria-label
  is the glance-summary (primary a11y path; trend-word = "climbing" | "steady" | "falling").
- `<polyline>` (point-per-day, N points) `stroke=var(--accent-success)` `stroke-width=1.5` `fill=none`
  + `<circle>` endpoint-dot at the last point.
- normalized viewBox (e.g. `0 0 100 30`, `preserveAspectRatio="none"`, mirroring the shipped `Bar`),
  rendered inline next to the WIP number, ≈80×20px.
- WCAG: `var(--accent-success)` stroke passes ≥3:1 against `--bg-elevated` in BOTH themes (#103/#425
  substrate) — verify at impl.
- A11y detail-fallback: `aria-describedby` → an `.sr-only` `<table>` of the N (date, wip) points (the
  full data, non-visual). The aria-label gives the trend-glance; the table gives the detail. (Matches
  the shipped `.sr-only` utility + the aria-describedby pattern in `MessageItem.tsx`.)

## Data dependency (dev-impl — flagged by dev at dev-OK)

The sparkline consumes a **WIP-history series** `Vec<(date, wip_count)>` (N daily points). The backend
does NOT produce this yet — `session::velocity::compute` returns aggregated 14-day metrics only, no
per-day WIP time-series. So #1237's dev-impl must ADD the backend data-source (extend velocity, or
derive daily WIP from the state-transition history) before the sparkline has data. That's a dev-impl
concern; the archetype above is the consuming **visual-spec**. Sparse/empty → the empty-state above.

## Inheritance for Epic #1236

Later chart-Features inherit: the 4px chart-card radius · the 1.5px stroke-weight · the chart-text→font-
token mapping · the `--space-*` chart-internal spacing · the semantic `var()` colors · the draw-in
entry-motion + reduced-motion fallback · the aria-label-glance + sr-only-table-detail a11y pattern.
This sparkline slice COMMITS them; later slices REUSE them (no re-deciding the chart language).
