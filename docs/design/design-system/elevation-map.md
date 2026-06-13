# Darcula elevation-map — design-of-record (2026-06-11)

The FOUNDATIONAL design-of-record for the Darcula design-system migration (Epic #1659):
the canonical SURFACE → ELEVATION-TOKEN mapping the app-slices migrate against. Fornax
(#1678 foundation) → Antlia (#1679 primitives) → Horologium (#1681 application) build to
this; #1682's audit-sweep verifies per-surface coverage against it.

**Framing — look-PRESERVING migration, not a redesign.** Grounded firsthand 2026-06-11
(CSS audit): the shipped renderer ALREADY follows this convention on ~91% of surfaces
(39/43, zero wrong-elevation usage). So this doc CODIFIES + ENFORCES the convention that
ships (Ryan-approved taste: contrast between sections, recessed chat, recessed board-group-
headers) and names the few ad-hoc colors to TOKENIZE — each app-slice migrates with NO
visible redesign (near-zero owner-rejection-risk; consistent-by-construction). It is NOT
the invented `#2b2b2b` palette of the early mockups — it grounds in the SHIPPED tokens.

## The elevation tokens (from `applications/electron/src/renderer/src/styles/tokens.css`)

| Token | Dark (`:root`) | Light (`[data-theme="light"]`) | Role |
|---|---|---|---|
| `--bg-base` | `rgb(26 26 26)` #1a1a1a | `rgb(255 255 255)` | recessed canvas / main content |
| `--bg-elevated` | `rgb(37 37 37)` #252525 | `rgb(245 245 245)` | panels / tool-windows |
| `--bg-highlighted` | `rgb(51 51 51)` #333 | `rgb(229 232 239)` | hover / selected / active |

Dual-theme already complete (tokens.css:81–102) — no Darcula-specific elevation tweak needed.

## The convention — IntelliJ editor-vs-tool-window elevation

The mental model (why it reads right + matches Ryan's taste):
- **Canvas / main content is RECESSED (`--bg-base`, darkest)** — the chat well + the
  titlebar. The thing you work IN sits lowest; panels float above it. (Ryan: "darker
  background for the chat area" + "areas that mark context headings.")
- **Panels / tool-windows are ELEVATED (`--bg-elevated`)** — the sidebar, the board rail.
  Tool-windows lift off the canvas → the section-contrast Ryan wants between regions.
- **Collapsible group-headers go BACK to base (`--bg-base`)** on the elevated panel — the
  board's disclosure-summaries. A recessed header on an elevated panel = the "darker header"
  on the board's collapsible sections Ryan likes.
- **Interactive states are HIGHLIGHTED (`--bg-highlighted`)** — hover / selected / active,
  uniformly, regardless of the surface's base elevation.

Nesting rule: elevation is RELATIVE. A card nested in base-level chat content uses
`--bg-elevated` (it lifts off the canvas); a recessed details-section inside an elevated
modal uses `--bg-base`. Read each surface against its PARENT, not absolutely.

## Canonical surface → token map (grounded; the audit-sweep target for #1682)

**Recessed canvas — `--bg-base`:** titlebar (AppShell.css:9) · chat well + channel-header
(Chat.css:7,23) · terminal view (TerminalView.css:8) · board group-headers /
disclosure-summaries (Board.css:114) · drill-in header (DrillIn.css:29).

**Elevated panels — `--bg-elevated`:** sidebar (Sidebar.css:10) · board rail (Board.css:10)
· chat input (Input.css:4) · nested cards (unfurl UnfurlCards.css:13 · code blocks
MessageList.css:167 · miniboard child Board.css:894) · floating UI (loading-pill
MessageList.css:16 · tooltip MessageList.css:360) · dialogs/modals (confirm-close
AppShell.css:82 · drill-in reject DrillIn.css:262 · error-boundary ErrorBoundary.css:7).

**Interactive states — `--bg-highlighted`:** every hover / selected / active across the
above (sidebar item Sidebar.css:71,75 · board row Board.css:221,225 · group-header hover
Board.css:118 · button hovers · input textarea Input.css:30). Uniform by convention.

## The drift — tokenization targets (the migration's systematization-ADD)

4 surfaces use ad-hoc raw colors instead of tokens (the ONLY drift; ~9%). These are
SEMANTIC-state colors the system hasn't tokenized — NOT elevation drift (no surface uses the
wrong elevation token). Design-call: they derive from the ACCENT palette / an overlay token,
NOT new `--bg-*` elevation tokens (elevation tokens stay for elevation):

| Surface | File:line | Current raw | → token (PIXEL-IDENTICAL value) |
|---|---|---|---|
| warning banner (chat) | Chat.css:12 | `rgb(58 47 26)` | `--surface-warning: rgb(58 47 26)` |
| warning banner (board) | Board.css:72 | `rgb(58 47 26)` | same `--surface-warning` (dedupe — identical value, two sites) |
| error toast | DrillIn.css:432 | `rgb(58 31 31)` | `--surface-error: rgb(58 31 31)` |
| modal scrim | DrillIn.css:279 | `rgb(0 0 0 / 50%)` | `--overlay-scrim: rgb(0 0 0 / 50%)` |

**Tokenize at the CURRENT value — NAMED, not re-colored (this stays look-preserving).** The
fix is to give the ad-hoc raw color a token NAME at its exact current value, so it renders
PIXEL-IDENTICAL — the systematization is the naming + dedupe (one `--surface-warning` for the
two identical sites), NOT a color change. If a current value already equals an accent-tint
(e.g. the warning-brown is a dark `--accent-warning` tint), the token MAY reference the accent
derivation — but only when the rendered value is identical. A DELIBERATE re-color to
accent-consistency (where it would visibly shift) is a SEPARATE, owner-confirmed refinement —
OUT of this look-preserving migration's scope (it carries owner-rejection-risk this migration
is built to avoid). (Shadows / accent-tint overlays / text-colors are NOT elevation drift —
out of scope here.)

## WCAG contrast — the a11y half (the foundation = elevation-map + this, ratified at Fornax)

Darcula's outcome is "consistent + ACCESSIBLE by construction." This doc maps BACKGROUNDS;
the coupled a11y requirement is text/foreground CONTRAST on each elevation. Fornax #1678's
token-ratification MUST verify (WCAG AA — 4.5:1 body text, 3:1 large/UI) for BOTH themes:

- text (`--text-primary` / `--text-secondary` / `--text-muted`) on each of `--bg-base`,
  `--bg-elevated`, `--bg-highlighted` — every text-on-elevation pair an actual surface uses;
- the new `--surface-warning` / `--surface-error` against their on-text + against the
  adjacent elevation (a warning banner sits on base/elevated — its text must clear AA);
- focus-ring / border tokens against the surfaces they bound (3:1 non-text).

Pixel-identical tokenization (above) means contrast is UNCHANGED for the 4 drift-sites (no new
contrast risk) — but the foundation isn't complete until the text-on-elevation pairs are
contrast-ratified. Whether this lives as a sibling contrast-map or a Fornax ratification-gate
is the foundation-slice's call; flagged here so the a11y-half is NEVER assumed. See
`cross-cutting-research.md` if a11y-research is gathered there.

### Computed contrast (2026-06-12, designer — the a11y-half, no longer assumed)

Ratios computed off the tokens.css values (script: `bpmux-designer/.scratch/darcula-contrast.mjs`).
AA bar: 4.5:1 body text, 3.0:1 large-text (≥24px / ≥18.66px-bold) + non-text UI. Result:
**mostly AA-clean; usage grounded 2026-06-12 (renderer CSS) → 3 CONFIRMED body-fails (each with
a look-preserving fix), 1 cleared:**

- **`fg-dimmer` — CONFIRMED AA-body fail wherever it carries info-text** (dark ~3.9-4.4 / light
  ~3.7-4.0): the grouped timestamp (MessageList.css:306, on bg-base 4.41/4.02), the
  closed-iteration badge (Board.css:446) + unscheduled-iteration title (Board.css:460, on
  bg-elevated 3.88/3.68) — all small info-bearing text < 4.5. FIX (Fornax): move these text-uses
  to `--fg-dim` (passes — elevated 5.65/5.40), keeping `fg-dimmer` for its NON-text uses (the
  drag-handle glyph Board.css:1049 = UI-3.0 ✓, the sidebar fills) — OR darken `fg-dimmer`.
- **light-theme `accent-info` — CONFIRMED AA-body fail on the elevated board** (3.76): the
  in-progress state-badge text (Board.css:342) + the "view details" 12px link (Board.css:379).
  Dark passes (7.65). The sibling state-badges (accent-brand/warning/success) all clear body in
  light (4.6-4.7) — only accent-info is short. FIX (Fornax): darken light `--accent-info`
  (rgb(2 132 199)) to clear 4.5 on `--bg-elevated`.
- **`msg-fg-muted` — CLEARED**: it's the to-other message-body text (MessageList.css:269), and
  the chat well is `--bg-base` where it passes (4.83). The elevated/highlighted shortfalls are
  theoretical — no muted body-text renders on those surfaces.
- **`@mention` pills — CONFIRMED AA-body fail for some roles** (this CORRECTS an earlier
  bg-highlighted guess — the real role-color TEXT usage is MessageBody.css:74-90: role-color on a
  `color-mix(role 15%, transparent)` tint over the chat well). The shared hue (role-text on a
  role-tint) drops contrast: **DARK @designer 4.14; LIGHT @ryan 4.03, @pm 4.09** — all < 4.5
  body. The 15% tint is the culprit — role-on-pure-base passes (light @ryan 4.92), the tint pulls
  it under. FIX (Fornax, designer-call): reduce the mention tint to ~8-10% (keeps the subtle
  highlight, restores ≥4.5 — look-preserving) OR shade the 3 failing role-texts in the mention
  context. (The role-as-avatar-bg MessageList.css:332-344: the initial is `--bg-base`-colored bold
  text on the role-color — CLEARED; it's the same pair as role↔bg-base, all ≥4.5 both themes,
  lowest dark @designer 5.03 / light @ryan 4.92.)

**Structural finding: `bg-highlighted` is the contrast FLOOR** — every pair bottoms out there,
so hover/selected/active states are where body-text is likeliest to drop below 4.5. Audit that
no BODY text lands on a highlighted state under 4.5.

**Semantic surfaces are text-clean**: `fg-primary` on `--surface-warning` (10.70) and
`--surface-error` (12.28) both pass AA-body. Their surface-vs-adjacent-elevation contrast is low
(warning 1.33/1.17, error 1.16/1.02) — EXPECTED for a subtle tint; ensure each banner/toast
carries a border or icon so it's distinguishable by more than color (the bg-tint alone is
neither a 3.0 UI-boundary nor needs to be).

Net: the elevation tokens + primary body-text are AA-sound; usage-grounding found 3 real body
fails (`fg-dimmer` info-text, light `--accent-info` on the board, `@mention` pills), each with a
named look-preserving fix above. Fornax applies the 3 fixes + ratifies. (Script + full ratio
table: `bpmux-designer/.scratch/darcula-contrast.mjs`.)

### Remediation values (computed 2026-06-12; `bpmux-designer/.scratch/darcula-remediation.mjs`)

Exact AA-body (4.5:1) remediations for the 3 fails — Fornax applies + ratifies against the real
surfaces:

1. **`fg-dimmer` info-text → `--fg-dim`** (no new value). `--fg-dim` clears every surface the
   text-uses touch (dark base 6.42 / elevated 5.65; light base 5.89 / elevated 5.40). Move the 3
   text-uses — MessageList.css:306 (grouped timestamp), Board.css:446 (closed-badge), :460
   (unscheduled-title) — to `--fg-dim`; keep `fg-dimmer` for its NON-text uses (drag-handle glyph,
   sidebar fills). Clean fix, no token-value change.
2. **light `--accent-info` darken**: rgb(2 132 199) → **rgb(2 108 163)** (~82% L). Clears the WORST
   surface accent-info text can land on — a hovered board-row (highlighted 4.66) + elevated 5.24 +
   base 5.71. Dark `--accent-info` already passes → light-only fix. Small, look-preserving shift (a
   marginally deeper blue).
3. **`@mention` tint 15% → ≤6%**: the `color-mix(role 15%, transparent)` tint shares the role's
   hue, dragging text contrast down. Clears at: dark @designer ≤9%, light @ryan ≤6%, light @pm ≤8%
   → **≤6% clears all three**. TRADEOFF: 6% is a faint highlight (may read weak). Fornax options:
   (a) ≤6% tint — simplest, faintens the pill; (b) keep the tint + a mention-specific role-text
   shade that clears 4.5 — preserves the pill, +tokens; (c) a NEUTRAL (non-hued) mention-tint —
   removes the shared-hue contrast-drag entirely. Recommend (c) or (a); owner-perceptible call.

## Per-slice application (look-preserving)

- **Fornax #1678 (foundation):** ratify the 3 elevation tokens + add the 3 semantic/overlay
  tokens above (`--surface-warning`, `--surface-error`, `--overlay-scrim`), dual-theme.
- **Antlia #1679 (primitives):** primitives consume the elevation tokens by their role
  (a card primitive = `--bg-elevated`; a recessed-header primitive = `--bg-base`).
- **Horologium #1681 (application):** point the app surfaces at the canonical tokens per the
  map above (already 91% there — mostly a no-op verify + the 4 drift-fixes).
- **#1682 audit-sweep (non-vacuous + complete):** the empty-diff must be a RENDERED/visual
  diff — the migrated app renders PIXEL-IDENTICAL to the shipped app (capture-compare), NOT a
  static "uses-a-token" check (a token's VALUE can change while the surface still "uses the
  token" → a look-shift passes vacuously). AND it must verify COMPLETENESS: enumerate ALL
  background surfaces in the renderer (grep raw colors + bg declarations) and assert each is
  (a) in this map and (b) tokenized — so an UN-MAPPED drifting surface is CAUGHT, not just
  that the mapped surfaces resolve.

## Falsifier (for the migration slices)

I would reject if: a migration introduces ANY visible look-change vs the shipped app,
INCLUDING the 4 drift-sites (tokenization is pixel-identical — a re-color there is a separate
owner-confirmed refinement, not this migration); OR the look-preserving proof is a static
"uses-a-token" check rather than a RENDERED capture-compare (vacuous — a value-change passes);
OR the audit-sweep checks only mapped-surfaces-resolve and MISSES an un-mapped drifting
surface (completeness gap); OR a surface moves to the WRONG elevation token (a panel onto
`--bg-base`, the chat onto `--bg-elevated`); OR the 4 drift-sites stay raw-color after
Horologium (un-tokenized); OR a new `--bg-*` ELEVATION token is invented for a SEMANTIC color
(warning/error/scrim are `--surface-*`/`--overlay-*`, not elevation); OR the WCAG-contrast
half is assumed rather than ratified (text-on-each-elevation + the semantic colors meeting AA,
both themes); OR the light-theme overrides break for any migrated surface.
