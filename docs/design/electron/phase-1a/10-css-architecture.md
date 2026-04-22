# phase-1a CSS architecture + frontend structure

Ryan's direction (2026-04-22) codifies styling philosophy + file structure (continuing from Battleship patterns).

## file structure (Battleship pattern)

Feature/domain folders at root — NOT a generic `components/` folder.

```
applications/electron/src/renderer/src/
├── App.tsx
├── App.test.tsx
├── AppShell.tsx              # window chrome (regions, layout)
├── AppShell.test.tsx
├── main.tsx
├── assets/                   # images, fonts, icons-as-svg
├── chat/                     # chat feature
│   ├── Chat.tsx
│   ├── Chat.css
│   ├── Chat.test.tsx
│   ├── MessageList.tsx
│   ├── MessageList.css
│   ├── Input.tsx
│   └── Input.css
├── board/                    # board feature
│   ├── Board.tsx
│   ├── Board.css
│   ├── StoryList.tsx
│   ├── StoryList.css
│   ├── DrillIn.tsx
│   └── DrillIn.css
├── sidebar/                  # sidebar feature
│   ├── Sidebar.tsx
│   └── Sidebar.css
├── ci/                       # CI glyph feature
│   ├── CiGlyph.tsx
│   └── CiGlyph.css
├── hooks/                    # shared React hooks (useWebSocket, etc.)
├── lib/                      # Result, AsyncResult, Maybe from Battleship
├── state/                    # global state (Zustand-free unless observed pain)
├── styles/                   # global-only
│   ├── reset.css             # Meyer reset
│   ├── tokens.css            # CSS variables for theming
│   └── shared.css            # shareable utility classes
├── transport/                # WS transport + auth handshake
└── test-setup.ts
```

**rules**:
- feature folders group Components + their CSS + tests — co-located
- tests live next to source (`.test.tsx`)
- no `components/` folder — features own their UI
- shared concerns factor to `hooks/`, `lib/`, `state/`, `styles/`, `transport/`
- `App.tsx` + `AppShell.tsx` at root — app entry + window chrome

## CSS layering (precedence, low → high)

1. **Meyer reset** (`styles/reset.css`) — starting-point reset. https://meyerweb.com/eric/tools/css/reset/
2. **Tag selectors** — only for the reset; never for component styling. Always FIRST items in any specifier list.
3. **Global tokens + shared classes** (`styles/tokens.css`, `styles/shared.css`) — CSS vars + utility classes
4. **Feature-local CSS** — co-located with feature files (`chat/Chat.css`, `board/Board.css`)

## CSS nesting (Ryan's direction: easier to read)

Use modern CSS nesting — natively supported in modern webviews (WebView2 Chromium, Safari 16.5+, WebKitGTK current). No preprocessor needed.

```css
.chat {
  background: var(--bg-base);
  display: flex;
  flex-direction: column;

  & .message {
    padding: var(--space-2);

    &.ryan {
      border-left: 2px solid var(--role-ryan);
    }

    & .timestamp {
      color: var(--fg-secondary);
      font-size: 0.85em;
    }
  }

  & .input {
    padding: var(--space-3);
    border-top: 1px solid var(--border-subtle);
  }
}
```

Read top-to-bottom as component structure — no CSS Module prefix soup.

## theming

`:root` = dark-mode default (phase-1a ships dark).
`[data-theme="light"]` = light-mode override (phase-1b).

Every component reads via `var(--token-name)`. No hardcoded color values.

## token categories (phase-1a dark-default)

**backgrounds**:
- `--bg-base` — main app background
- `--bg-elevated` — panels (sidebar, board, chat)
- `--bg-highlighted` — selected rows, hovered items

**foregrounds**:
- `--fg-primary` — body text
- `--fg-secondary` — metadata (timestamps, labels)
- `--fg-dim` — hint text, inactive

**accents**:
- `--accent-brand` — primary action (accept button, active selection)
- `--accent-success` — CI green, success badges
- `--accent-warning` — rework, pending
- `--accent-error` — CI red, reject, errors
- `--accent-info` — neutral info, links

**borders**:
- `--border-subtle` — panel dividers
- `--border-strong` — focus rings, selected row borders

**role colors** (for messages):
- `--role-ryan`
- `--role-designer`
- `--role-dev`
- `--role-pm`

**spacing scale — base 8** (px units; same scale used for spacing + heights + widths):

| Token | Value | Role |
|---|---|---|
| `--space-2` | 2px | ¼ step (tight borders, fine-detail spacing) |
| `--space-4` | 4px | ½ step (small gaps inside compact elements) |
| `--space-6` | 6px | ¾ step (between ½ and base — tight-but-not-cramped) |
| `--space-8` | 8px | base unit |
| `--space-16` | 16px | 2× |
| `--space-24` | 24px | 3× |
| `--space-32` | 32px | 4× |
| `--space-40` | 40px | 5× |
| `--space-48` | 48px | 6× |
| `--space-56` | 56px | 7× |
| `--space-64` | 64px | 8× |

extend as needed (always multiple of 8 after base; fractional steps 2/4/6 only between 0 and 8).

## units

- **text sizes: `rem`** — scales with root font-size; respects user accessibility settings
- **borders, padding, margin, widths, heights, gaps: `px`** — predictable pixel-accurate layout
- **do NOT use**: `em` (inherited scaling surprises), `%` except where intentional (e.g., flex-basis), `vh`/`vw` except for full-viewport needs

## layout primitives

- **grid** — 2D layouts (window regions, story-row columns, metric dashboards)
- **flexbox** — 1D layouts (rows, columns, alignment)
- **NOT**: float, absolute/relative positioning (except for tooltip/overlay edge cases)

## spacing: prefer gap + padding, NOT margin

- **container controls spacing** via `gap` (grid/flex) + `padding`
- margin is last-resort — only for truly unusual cases (e.g., aligning one item against a separate context)
- why: gap + padding make spacing a CONTAINER property; consistent, no margin-collapse surprises, easier to reason about
- if you reach for margin, ask: "could this be gap on the parent, or padding on the container?" — usually yes

## responsive

Electron runs on resizable desktop windows. Handle breadth gracefully, never assume a fixed size.

**patterns**:
- **fluid layouts**: grid/flex proportional sizing (fr units, flex-grow); avoid fixed-pixel widths except for chrome (sidebar ~220px, board ~360px as default starting widths)
- **min/max/clamp**: use `min-width` + `max-width` on regions; `clamp()` for responsive typography if needed (phase-1b)
- **container queries** (`@container`) > media queries for component-level responsiveness — new CSS, supported in current webviews (WebView2 Chromium, Safari 16.4+, webkit2gtk current)
- **text reflows naturally** — no `white-space: nowrap` outside chrome; no fixed-width text containers

**window-width breakpoints** (app-shell level):
- **≥ 1200px**: full 3-region (sidebar + chat + board) — ideal
- **800-1199px**: 3-region with board-panel possibly narrower, or sidebar-icons-only toggle
- **< 800px**: board panel auto-hides; chat + sidebar remain (per `00-window-layout.md`)
- **< 500px**: sidebar collapses to icons-only; chat full-width (edge-case; Electron likely never that narrow)

**component-level**:
- use `@container` queries inside feature CSS when a component needs to adapt to its parent region's width (e.g., StoryRow compacts differently at narrow panel widths)
- no assumptions: component should work at any reasonable width within parent container

**accessibility as responsive**:
- semantic HTML (`<nav>`, `<main>`, `<aside>`, `<button>`, `<header>`, `<footer>`) — screen readers + keyboard nav work naturally
- `aria-label` on icon-only buttons + status indicators
- keyboard focus visible (`:focus-visible` ring using `--border-strong` var)
- color is never the sole signal (CI glyph shape + color; selection bar + border + background bg; etc)

**motion respect**:
- `@media (prefers-reduced-motion: reduce)` disables non-essential animations
- default motion should be subtle: 150-250ms transitions; no spinning/bouncing unless state-semantic (e.g., CI Running spinner)

**testability**:
- Playwright tests should resize window (360x640 narrow, 1024x768 medium, 1920x1080 wide) + verify layout holds
- phase-1a ships at 1024x768 default; phase-1b polish exercises narrower/wider

## example token file (for dev reference)

```css
:root {
  /* dark default */
  --bg-base: #1a1a1a;
  --bg-elevated: #252525;
  --bg-highlighted: #333;
  --fg-primary: #e8e8e8;
  --fg-secondary: #b0b0b0;
  --fg-dim: #808080;
  --accent-brand: #6aa0ff;
  --accent-success: #3fb950;
  --accent-warning: #d4a13c;
  --accent-error: #ff6b6b;
  --accent-info: #4fc3f7;
  --border-subtle: #333;
  --border-strong: #555;
  --role-ryan: #ffd54f;
  --role-designer: #b368e9;
  --role-dev: #4fc3f7;
  --role-pm: #81c784;
  /* spacing: base 8, fractional steps 2/4/6 below base */
  --space-2: 2px;
  --space-4: 4px;
  --space-6: 6px;
  --space-8: 8px;
  --space-16: 16px;
  --space-24: 24px;
  --space-32: 32px;
  --space-40: 40px;
  --space-48: 48px;
  --space-56: 56px;
  --space-64: 64px;
}

[data-theme="light"] {
  /* phase-1b override block */
}
```

(Dev picks final hex values; designer provides token names + categorical mapping.)

## HTML5 semantic elements — NO div or span

Ryan's direction: use semantic HTML; avoid `<div>` and `<span>` unless there's no semantic alternative.

**preferred tags**:
- **structural**: `<header>`, `<footer>`, `<main>`, `<nav>`, `<aside>`, `<section>`, `<article>`
- **lists**: `<ul>`/`<ol>`/`<li>` for lists; `<dl>`/`<dt>`/`<dd>` for term-definition
- **text**: `<p>`, `<h1>`-`<h6>` for headings, `<strong>`/`<em>` for emphasis, `<small>` for fine-print, `<time>` for timestamps, `<code>`/`<pre>` for code
- **interactive**: `<button>` (not `<a>` for actions), `<a>` (for navigation), `<input>`, `<label>`, `<form>`, `<textarea>`
- **media + collapse**: `<figure>`, `<figcaption>`, `<details>`, `<summary>`
- **table**: `<table>`, `<thead>`, `<tbody>`, `<tr>`, `<td>`, `<th>` for tabular data

**mapping to bpmux surfaces**:
- App shell → `<main>` container; `<header>` for titlebar; `<aside>` for sidebar + board; `<footer>` for chat input
- Chat messages → `<article>` per message; `<time>` for timestamp; `<cite>` for role name (or just `<strong>`)
- Board story list → `<ul>` with `<li>` per story; `<article>` for drill-in body
- CI glyph → `<button>` (clickable)
- Empty states → `<p>` with italic class rather than `<div>` wrapper

**when `<div>` / `<span>` IS allowed**:
- truly non-semantic layout wrappers when no semantic tag applies (rare)
- CSS grid anonymous cells where children are the semantic elements
- prefer container semantics (`<section>`, `<article>`, `<nav>`, `<aside>`) if they fit

**why**: screen readers + accessibility tooling + document outline all leverage semantic HTML. Tag choice IS a UX decision, not just a class-hook.

## ARIA — minimal, only where native semantics can't deliver

**First rule of ARIA use** (per WAI-ARIA / MDN): if you can use a native HTML element or attribute with the semantics and behavior you need, use it — DO NOT repurpose an element + add ARIA role.

- `<button>` not `<div role="button">`
- `<nav>` not `<div role="navigation">`
- `<main>` not `<div role="main">`
- `<ul><li>` not `<div role="list"><div role="listitem">`
- `<header>` / `<footer>` / `<aside>` — don't add roles, they come built-in

**when ARIA IS warranted**:
- `aria-label` on icon-only buttons (no visible text): `<button aria-label="Toggle board panel">⌘P</button>`
- `aria-live` on regions announcing dynamic content (chat message arrival, CI state transition): `<section aria-live="polite">`
- `aria-expanded` / `aria-controls` on collapsible state-group headers (already implicit with `<details>` + `<summary>` — prefer native)
- `aria-current` for the active channel in sidebar
- `aria-describedby` for supplementary descriptions

**reference**: https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA

**bpmux-specific applications**:
- chat message region → `<section aria-live="polite">` so screen readers announce new messages
- CI glyph (button) → `aria-label` with full state ("CI passing on main. Click to view on GitHub Actions.")
- drill-in panel → `<article>` with implicit role; no ARIA needed
- story list items → `<li>` inside `<ul>` — no roles, native semantics deliver
- selected story → `aria-current="true"` on the `<li>` (or `<a>` if story rows are anchors)

## anti-patterns (do not)

- Tailwind utility classes (Ryan's no-libs rule)
- CSS-in-JS (styled-components, emotion, etc.)
- `components/` folder bucket — organize by feature
- Hardcoded color values — always through `var(--token)`
- Styling raw tags outside the reset — always use classes
- Global CSS reaching into feature internals — feature CSS owns its DOM
- CSS preprocessors (Sass, Less) — CSS nesting is native now; no build-step needed
- `<div>` / `<span>` when a semantic tag fits
- `<button>` styled as `<a>` or vice versa — match semantic to action (button = triggers action, anchor = navigates)
