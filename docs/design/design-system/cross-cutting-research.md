# Design-system — cross-cutting research (grid · responsive · accessibility · keyboard)

The dimensions beyond the button/modal slice (`button-modal-research.md`), grounded in the
reference systems (Carbon 2x-grid, WCAG, GitLab/VA) + IntelliJ — per Ryan (2026-06-06):
"the design systems have guidance for ALL visible components — the grid, responsiveness,
accessibility, keyboard affordances; IntelliJ can help." These are the cross-cutting
principles EVERY component carries (baked, not bolted on).

## GRID (Carbon 2x-grid + our existing tokens)
- **8px mini-unit base** — matches our `--space-8` + Ryan's 8px/4px research grid. Spacing scale =
  multiples of 2/4/8; our tokens already are (2/4/6/8/12/16/24/32/40/48/56/64 ✓). Keep.
- **Layout** = a responsive 16-column grid (Carbon 2x). Default 32px gutter; **wide / narrow /
  condensed** gutter variants tune density per region.
- **Fluid** (column-count constant within a breakpoint, unit scales) vs **fixed** (fixed unit, tile+wrap).
  For bpmux: fixed-density panels + the board/sidebar/chat/terminal as flex/grid regions that reflow.

## RESPONSIVE
- Breakpoints (Carbon ships 5). For an Electron desktop app the range is window-resize (narrower than
  web), but the panels (sidebar / board / chat / terminal) must reflow gracefully on resize.
- **Modal %-widths** (Ryan's research): XS 24–48% · M (default) 48–60% · L 60–80%.
- **Density** via the gutter-structure (wide/narrow/condensed) — the board/chat are dense; tune per region.

## ACCESSIBILITY (WCAG — our HTML5-semantic / first-rule-of-ARIA canon)
- **Focus-visible** (WCAG 2.4.7): a visible focus-ring — **3px, ≥3:1 contrast** against adjacent — on
  EVERY interactive element. (We have `:focus-visible` accents; standardize the ring as a token.)
- **Target size** ≥ **44×44px** (Ryan's research 44px; Apple 44 / Material 48; WCAG-2.2 min 24×24).
  Buttons / glyphs / rows must meet it — invisible-padding is fine.
- **Contrast** ≥ 4.5:1 text (AA), ≥ 3:1 large-text/UI. The Darcula palette MUST pass — verify
  `#bbbbbb` on `#2b2b2b` + each accent (a contrast-audit is part of the palette slice #1660).
- **ARIA**: native HTML5 semantics FIRST (`<button>` / `<dialog>` / `<nav>` over ARIA-on-div) — our
  existing canon. ARIA only where native can't express it.
- High-impact components to harden first: **buttons · forms · modals · navigation · data-tables**
  (= our board / sidebar).

## KEYBOARD AFFORDANCES (IntelliJ + WCAG)
- **Full keyboard operability** (WCAG 2.1.1): Tab / Shift-Tab / Enter / Space / Escape on every interaction.
- **Visible focus at all times** (the focus-ring above) — sighted keyboard users must always see where focus is.
- Extend the **keyboard-reorder** we already have (Ctrl+Shift+Arrow on the drag-lists) as the pattern.
- **IntelliJ keymap inspiration**: discoverable shortcuts, focus-cycling between panes, the
  command-palette pattern (we have a command-palette arc) — keyboard-first nav of board / sidebar / chat.

## How this maps to #1659's cross-cutting slices
Each is a real cross-cutting slice (PM's #1659 structure), carried into every per-component mockup:
- **grid + density** — the 8px 2x-grid + gutter-structure.
- **responsiveness** — breakpoints + panel-reflow + modal %-widths.
- **accessibility / WCAG** — focus-ring token · 44px targets · contrast-audit · native-semantics.
- **keyboard-affordances** — full operability · visible focus · IntelliJ-keymap patterns.

The per-component mockups bake these in by construction (the focus-ring, the 44px target, the
keyboard-nav, the responsive-reflow are part of each component, not a later pass).

## Sources
- Carbon 2x-grid + spacing — carbondesignsystem.com/elements/2x-grid · /elements/spacing
- WCAG — 2.4.7 focus-visible · 2.1.1 keyboard · 2.5.5 target-size (w3.org/WAI)
- Ryan's button/modal structural research — `button-modal-research.md` (GitLab / Carbon / VA)
- a11y design-system guidance (a11ypros, level-access, vispero focus-indicators)
