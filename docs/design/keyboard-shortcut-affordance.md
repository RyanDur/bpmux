# Keyboard-shortcut affordance — discover · define · edit

Owner-DIRECTED 2026-06-13 (Ryan: "the app needs keyboard shortcut affordance… I pointed you at the intellij app and how the jetbrains people designed how shortcuts are defined and how they are edited"). Designer **brainstorm-seed** — deliberately THIN (go-wide-then-converge; validate the riskiest assumption cheapest). PM materializes + sequences; dev tech-scopes; we converge with Ryan. **Future-readiness, not current-iteration.**

## The problem (grounded in the shipped app, not a phantom)

bpmux is a keyboard-driven dev tool, but its shortcuts are **invisible, scattered, and uneditable** (verified against the running app, 2026-06-13):

- **Invisible:** app-level `Ctrl/Cmd+P` (toggle board) and `Ctrl+Shift+M` (metrics overlay) have **no UI that announces them** — no menu (autoHideMenuBar), no palette, no hints. A user can't discover them. The only shortcut hint anywhere is "Cmd+Enter to send" in the comment placeholder.
- **Scattered:** every shortcut is an inline `keydown` handler in its own component (`AppShell.tsx`, `Comments.tsx`, `DrillIn.tsx`, `BoardResizeHandle.tsx`, `TerminalView.tsx`). There is **no central registry** — no shared table of action→binding.
- **Uneditable + collision-prone:** no keymap settings, no conflict-detection. Adding a shortcut means remembering every other one by hand (discipline, not mechanism).

So: the shortcuts a power-user would live by can't be found, can't be changed, and can't be safely added to.

## The owner's reference (JetBrains / IntelliJ keymap)

Three jobs, the model Ryan pointed at:
- **Discover** — a searchable keymap; **Find Action** palette (`⌘⇧A`) that shows each action's shortcut inline; shortcut hints shown next to actions in menus/tooltips; "find by shortcut" (press the keys → see what's bound).
- **Define** — one central keymap: every action listed with its default binding.
- **Edit** — per-action add/remove a shortcut, with **conflict-detection**; switchable keymaps (macOS / default / Emacs…); persisted.

## Solution-spectrum (go-wide; thinnest → richest; for the converge, NOT decided)

| | what | benefits | cost |
|---|---|---|---|
| **A** | **central action registry + visible hints** — declare every shortcut in one table; surface hints in the UI (tooltips / a help line) | kills "invisible"; the substrate every other slice needs; mechanism replaces scattered discipline | **cheapest** — a registry + read it where shortcuts already fire |
| B | **command palette** (`⌘⇧P`) — search actions by name, run them, **see the shortcut** inline | the IntelliJ-signature discovery+run; immediately useful in a keyboard-driven tool | light–medium (a palette UI over the registry) |
| C | **keymap settings view** — searchable list of all actions + bindings; find-by-shortcut | the full "what's bound" surface | medium |
| D | **edit + conflict-detection + custom/switchable keymaps**, persisted (localStorage pattern already proven) | rebind to taste, safely; platform keymaps | heaviest |

## Riskiest assumption + thinnest-first (Lean)

**Leap-of-faith to validate:** *does surfacing + centralizing shortcuts actually change behavior* — do people discover and use them, and does a registry make adding shortcuts safe? Editing/custom-keymaps (D) are cheap to defer; this determines whether the affordance earns its build.

**Thinnest validating slice = A + B (registry + command palette):** the registry is the foundation (one declared table, replacing the scattered inline handlers — a substrate-quality win on its own); the palette is the cheapest surface that makes the assumption testable — search an action, see its binding, run it. Together they (1) make invisible shortcuts findable, (2) give one home for every binding, (3) seed conflict-detection (a registry can be checked). Defer the settings-editor (C/D) until real use says rebinding is wanted — don't build the editor on an assumption.

## Desirability · Feasibility · Viability

- **Desirability (designer):** the affordance + the mockup (the palette + the hint-surface) — mine, when prioritized.
- **Feasibility (dev):** how big is the registry refactor (lift the inline handlers into one declarative table without regressing behavior)? the palette over it? custom-bind persistence reuses the localStorage hooks (`useBoardWidthStorage`-style). dev tech-scopes.
- **Viability (Ryan + PM):** earns the build vs. later, and where it sequences (a future iteration, not Crux).

## Open questions (for the converge + dev tech-scope)

- **Registry shape:** a declarative table (action id → label → default binding → handler) the components subscribe to, or codegen? (mechanism-over-discipline: the table is the single source.)
- **Palette vs settings first:** palette (discover+run, B) is the cheaper validator; settings (C) is the fuller "define/edit" home. Which first?
- **Terminal passthrough:** xterm.js owns its own keys in `TerminalView` — the registry must NOT capture keystrokes meant for the terminal (editable-context check already exists app-level).
- **Platform:** `Ctrl` (Win/Linux) vs `Cmd` (macOS) already handled via `ctrlKey || metaKey`; a registry should encode this once, not per-binding.
- **Conflict policy:** on a collision, warn-and-block, warn-and-override, or both-fire? (IntelliJ warns + lets you reassign.)
- **Where it attaches:** the titlebar (next to the theme toggle) is the only settings-ish surface today — does a keymap page live there, in the sidebar, or only as the palette?
