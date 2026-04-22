# phase-1a window layout

## regions

```
┌─ bpmux ──────────────────────────────────── ✓ CI ── ≡ ┐
│                                                       │
│ ┌─ sidebar ──┐ ┌─ chat ────────────┐ ┌─ board* ─────┐ │
│ │ broadcast  │ │ [message list]    │ │ Ready        │ │
│ │ ▾ direct   │ │                   │ │ #123 title…  │ │
│ │   designer │ │                   │ │ In Progress  │ │
│ │   dev      │ │                   │ │ …            │ │
│ │   pm       │ │                   │ │              │ │
│ │ ▾ groups   │ │                   │ │ *open via    │ │
│ │   dev+pm   │ │                   │ │  Ctrl+P or   │ │
│ │ ▸ system   │ │                   │ │  menu; hides │ │
│ └────────────┘ │                   │ │  when closed │ │
│                │                   │ └──────────────┘ │
│                └───────────────────┘                   │
│                                                       │
│ ┌─ chat input ────────────────────────────────────┐   │
│ │ type here…                                      │   │
│ └─────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────┘
```

## region responsibilities

- **titlebar**: app name + CI status glyph (clickable → GitHub Actions) + OS window controls
- **sidebar** (left, ~220 cols fixed at 1920×1080; resize-to-taste): channel tree (broadcast / direct / groups / system). Click selects; Enter confirms.
- **chat** (center, flex-grow): message list above, input at bottom. Messages stream bottom-up.
- **board** (right, ~360 cols fixed when open): story list + drill-in. Toggled via Ctrl+P or View menu.
- **no global status bar**: presence moves into sidebar (per-peer badges); CI status moves into titlebar.

## responsive behavior

- minimum window width: **800**. below that, board panel forces-hide (chat + sidebar only).
- sidebar can collapse to icons-only (~60px) via toggle; preserves chat width for narrow windows.
- all panels resize proportionally when window resizes; no jumpy re-layout.

## collaboration invariant (load-bearing per `feedback_collaboration_visibility_inviolable`)

**chat input is ALWAYS reachable.** No state (drill-in open, board focused, settings modal, anything) may block typing or submitting to chat. This is non-negotiable — bpmux is an AI+human collaboration tool; the chat must never be taken away.

## focus model

- exactly one region has keyboard focus at a time: sidebar | chat-input | board | drill-in
- Tab cycles focus between regions (sidebar → chat → board if open)
- Chat input auto-focuses on app launch and after sending a message
- Escape from any non-chat region returns focus to chat input

## theme

- phase-1a ships **one default theme**. Dark mode + theme system slot in phase-1b.
- base color tokens documented per surface-spec file; picks defined once at `90-tokens.md` (to be written).
