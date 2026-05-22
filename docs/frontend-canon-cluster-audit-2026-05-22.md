# Frontend canon-cluster audit — 2026-05-22

Caelum Slice 9 (#1486), Epic #1477 terminal-verification slice. Full-renderer
audit confirming the frontend canon-cluster is at a substrate-enforced
zero-violation baseline. Per-axis: canon-memory + enforcing config-rule +
command run + observed result + carve-outs.

Scope: `applications/electron/src/renderer/src` — 21 CSS files, all `*.tsx`.
Audit found + fixed 2 pre-existing violations the per-surface slices missed
(see "Findings fixed"); after those, every axis is clean.

## Axis results

### Axis 1 + 6 — stylelint (CSS units / colors)
- Canon: `css_units_canon_rem_px_percent` (rem fonts, px borders/radii) +
  `css_shared_at_root_local_for_structural` / `css_is_for_presentation_no_cheating`
  (no hard-coded color in component CSS).
- Enforcing config: `.stylelintrc.json` — `declaration-property-unit-allowed-list`
  (font-size→rem; border/radius/outline→px), `color-no-hex: true`,
  `color-named: "never"`.
- Command: `npm run lint:css` (`stylelint "src/**/*.css"`).
- Result: **exit 0** across 21 renderer CSS files. Clean.
- Carve-out: `src/renderer/src/styles/*.css` (reset/tokens) exempt from the
  tag-selector ban (`.stylelintrc.json` `selector-max-type` override) —
  deliberate, reset CSS legitimately uses tag selectors. Not a violation.
- Spacing supplement (stylelint does NOT mechanically ban spacing literals):
  `var(--space-N, Npx)` token-with-fallback (Board.css 400/411/412) and
  `margin: -1px` visually-hidden-clip pattern (Tabs/FilterPills) are
  legitimate. One sub-token nudge `Board.css:1100 (padding: 0 2px)` is raw px
  but is NOT a stylelint violation (no spacing-token rule exists) — noted for
  a future spacing-token rule (Octans-class), not a canon-violation today.

### Axis 2 + 4 + 5 — eslint (div/span ban + ARIA)
- Canon: `html5_semantic_no_divs_spans` + `first_rule_of_aria`.
- Enforcing config: `eslint.config.mjs` `no-restricted-syntax` bans `<div>`/`<span>`
  JSXOpeningElements in `src/renderer/src/**/*.tsx` (excluding `*.test.tsx`,
  `fixtures/**`); `eslint-plugin-jsx-a11y` (recommended) covers role-misuse.
- Command: `npm run lint` (`eslint --cache .`).
- Result: **exit 0, 0 errors**. Clean. (169 warnings remain — all
  NON-canon-cluster: `testing-library/*` 134, `prettier/prettier` 19,
  `react-hooks/exhaustive-deps` 4, `jest-dom` 1 — pre-existing test-quality,
  outside this audit's canon-cluster scope.)

### Axis 3 — JSX-whitespace-hack `{' '}`
- Canon: `jsx_whitespace_hack_is_offensive` (use CSS layout primitives, not
  `{' '}`). No lint rule — grep-only.
- Command: `rg "\{' '\}|\{\" \"\}|&nbsp;" src/renderer/src --glob '*.tsx'
  --glob '!**/*.test.tsx' --glob '!**/fixtures/**'`.
- Result: **0 hits** after the AppShell fix (see Findings). Clean.

### Axis 4 — `<div>`/`<span>` in production renderer
- Command: `rg "<div|<span" src/renderer/src --glob '*.tsx'
  --glob '!**/*.test.tsx' --glob '!**/fixtures/**'`.
- Result: **0 hits** after the Metrics fix (see Findings). Clean.
  (Test/fixture `*.tsx` are intentionally eslint-exempt — not in scope.)

### Axis 5 — `role=` patches over native HTML5
- Canon: `first_rule_of_aria` (native HTML over role= where a native element
  exists).
- Command: `rg 'role="' src/renderer/src --glob '*.tsx' --glob '!**/*.test.tsx'`.
- Result: 12 hits, ALL legitimate (no native HTML5 equivalent), 0 violations
  after path/role discrimination:
  - `role="alert"` ×3 (ErrorBoundary, Board, Metrics) — aria-live alert region; no native element.
  - `role="img"` ×3 (StoryTypeGlyph, ClassGlyph ×2) — `<svg>` + aria-label, the canonical accessible-icon pattern.
  - `role="slider"` (BoardResizeHandle) — custom resize-handle; `input[type=range]` is a different control.
  - `role="tooltip"` (MessageItem), `role="menu"/"none"/"menuitem"` (Reactions), `role="log"` (MessageList) — no native equivalent.

### Axis 7 — audit-trail doc
- This document. Present.

### Axis 8 — precondition (Slices 1-8 state)
- 6 of 8 accepted/retired: Slice 1 #1478 accepted; Slice 4 #1481 accepted;
  Slices 5-8 #1482/#1483/#1484/#1485 retired (shipped-elsewhere).
- **Slices 2 (#1479) + 3 (#1480) are delivered, not yet accepted** — their
  code IS on `main` (delivered = merged) and is therefore included in this
  audit. Per designer's full-renderer reframe (the audit verifies the actual
  code-on-main, the meaningful baseline, broader than literal slice-acceptance),
  this is the correct scope. Surfaced honestly: if #1479/#1480 are reworked
  before acceptance, re-run this audit against the changed code.

## Findings fixed (pre-existing, slice-missed)

The slices applied canon per-surface; two surfaces were missed and carried
real violations until this audit:

1. `metrics/Metrics.tsx` — `<div className="metrics-tile">` (no-restricted-syntax
   error). HTML5's only sanctioned dt/dd grouper is `<div>`-in-`<dl>`, but the
   canon bans `<div>` and eslint-disable is disallowed → restructured to
   `<section className="metrics-grid">` containing per-tile
   `<dl className="metrics-tile">` (each holds one dt/dd). Class-based CSS
   unaffected; metrics e2e green.
2. `AppShell.tsx:157` — `bpmux{' '}` JSX-whitespace-hack. `.titlebar-app` is
   already `inline-flex` with `gap: var(--space-4)` (AppShell.css:27-30), so
   the `{' '}` was redundant double-spacing. Removed; the flex gap is the
   single spacing owner. No test asserts the titlebar textContent.

## Verification commands (reproducible)

```
cd applications/electron
npm run lint:css   # stylelint — exit 0
npm run lint       # eslint — exit 0, 0 errors
rg "\{' '\}|\{\" \"\}|&nbsp;" src/renderer/src --glob '*.tsx' --glob '!**/*.test.tsx' --glob '!**/fixtures/**'
rg "<div|<span"    src/renderer/src --glob '*.tsx' --glob '!**/*.test.tsx' --glob '!**/fixtures/**'
```

Baseline: substrate-mechanisms (stylelint + eslint custom rules from Slice 1
#1478 + #1493 + #1502) now mechanically prevent div/span, hex color, and
wrong-unit recidivism. The canon is enforced at write-time, not just held in
memory — future canon-application ships against this zero-debt substrate.
