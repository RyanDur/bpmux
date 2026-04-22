# phase-1a CI glyph + click-through

## placement

- top-right of titlebar area, left of OS window controls
- single compact indicator: icon + optional label

## states (four)

| state | glyph | color | meaning |
|---|---|---|---|
| Green | ✓ | green | last CI run on main passed |
| Red | ✗ | red | last CI run on main failed |
| Running | ◌ (spinner) | amber | CI in progress |
| Unknown | ? | dim gray | not yet polled OR poll failed (disambiguation deferred to phase-1b) |

- glyph updates reactively as poller broadcasts state changes
- small + unobtrusive; never animated except Running (subtle spin)

## click behavior

- click opens `https://github.com/RyanDur/bpmux/actions` in user's default browser
- OS-native open (Electron `shell.openExternal`)
- no loading state; browser handles from there

## hover (phase-1a: minimal)

- tooltip shows current state text ("CI passing on main" / "CI failed on main" / "CI running" / "CI state unknown")
- phase-1b: richer hover-preview (commit sha + run duration + last-updated time)

## accessibility

- aria-label with current state + action ("CI: passing. Click to view details on GitHub Actions.")
- keyboard: Tab reaches the glyph; Enter activates click
- color is not sole signal (glyph shape differs per state)

## implementation-agnostic behavior

- driven by existing backend `get_ci_state` MCP + broadcast signal
- Electron UI subscribes to state updates; renders glyph on change
- no bpmux-side CI dashboard — GitHub is canonical per `external-canonical-state-in-tui`

## phase-1b defers

- hover-preview with last-run details
- Unknown-state disambiguation (never-polled vs poll-failed)
- per-workflow filtering (currently shows latest run regardless of workflow)
- click-to-most-recent-run (deep link vs actions-index)
- branch switching (currently locked to main)
