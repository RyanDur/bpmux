# phase-1a Electron specs

Designer specs for the MVP that replaces the TUI. Scope = minimum for Ryan to switch primary workflow off terminal.

## specs in this directory

- `00-window-layout.md` — window regions, focus model, collaboration invariant
- `01-chat.md` — message list + input + emacs/readline bindings + @mention addressing + timestamps
- `02-board.md` — story list + drill-in + drag-reorder
- `03-accept-reject.md` — owner-gate actions + acceptance-walk visible chrome
- `04-ci-glyph.md` — status indicator + click-to-GitHub
- `10-css-architecture.md` — Meyer reset + tokens + co-located component CSS + dark-default theming

## phase-1a scope (per dev's MVP shape)

- Electron spawns Rust backend via WS
- Chat: broadcast + DM, send plain text
- Board: stories by state + clickable drill-in
- Drill-in: body + labels + state + assignees (plain rendering)
- Accept / Reject: owner-gated buttons in drill-in
- CI glyph: state + click-to-GitHub

## explicit phase-1b deferrals

- rich markdown (code highlight, tables, images)
- metric charts
- dark mode + theme system
- native OS notifications
- emacs input bindings (browser defaults first; emacs bindings as polish)
- Slack richness (emoji reactions, pins, saved, unfurls)
- drag-drop files
- search UI
- typing indicators
- timeline audit-trail (C in matrix)
- team-accept badges (D in matrix)
- context menus

## collaboration-continuity invariant

**chat input is always reachable + typeable.** Any future surface must preserve this. See `00-window-layout.md` for the guarantee.

## quality bar (per dev's phase-1a MVP principle)

- each feature done-done: works + has a test before moving on
- WS reconnect works reliably (network blip, backend restart)
- Rust child cleanup on Electron quit (no orphan processes)
- Playwright smoke test per feature — agents can drive end-to-end

## not in phase-1a

- slash commands (retired entirely)
- command palette (Cmd+K = clear input, not a palette)
- in-app timeline + audit badges
- @mention autocomplete (can type `@dev` inline; parser resolves on send)
