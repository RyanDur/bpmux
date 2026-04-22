# phase-1a chat surface

## layout

```
┌─ chat ─────────────────────────────┐
│ [all] · 201 hidden                 │  ← filter + class-hidden counter row
├────────────────────────────────────┤
│ designer 14:22  broadcast          │
│   message text here with wrap      │
│   and multi-line continuation      │
│                                    │
│ dev 14:24  [→ designer]            │  ← addressee signal (colored/bordered)
│   replying to something            │
│                                    │
│ [scrolling up loads more]          │
├────────────────────────────────────┤
│ ryan > type here, Enter to send    │  ← input always reachable
└────────────────────────────────────┘
```

## message row

- **role name** (colored per role) + **timestamp** (dim) + **channel/scope badge** ([broadcast] / [→ designer] / [designer+dev])
- **body** wraps naturally; no truncation; code blocks plain monospace for phase-1a
- **addressed-to-me**: left-border accent color (ryan's messages addressed via `@role` or DM)

## timestamps

- **viewer's local time + timezone abbreviation** per message: e.g., `8:35 PM CDT`
- agents reading via bridge (server-process typically UTC) see same underlying UTC-timestamp rendered `01:35 UTC`
- everyone reads with disambiguation via the TZ suffix — same event, different display per viewer, unambiguous
- implementation: capture `Intl.DateTimeFormat().resolvedOptions().timeZone` at render time; format timestamp in that zone with short TZ abbreviation

## addressing (Slack-convention, no slash)

- `@role` inline in message text → attention signal (addressee field in backend); still sent to broadcast
- DM/group: user clicks recipient in sidebar → chat switches to that channel → messages scoped automatically
- no `/to` syntax

## input area

- single-line default; auto-grows to multi-line as text overflows (up to ~6 rows before scroll)
- Enter submits; Shift+Enter adds newline
- **emacs/readline bindings** (per Ryan's direction):
  - Ctrl+A / Ctrl+E: beginning/end of line
  - Ctrl+B / Ctrl+F: char backward/forward
  - Alt+B / Alt+F: word backward/forward
  - Ctrl+K: kill to end of line
  - Ctrl+U: kill to beginning of line
  - Ctrl+W: delete previous word
  - Ctrl+Y: yank (paste last kill)
  - Ctrl+P / Up: history previous
  - Ctrl+N / Down: history next
  - Ctrl+R: incremental history search
  - **Cmd+K: clear entire input** (per Ryan)

## class filter

- top-bar dropdown or pills: `[all]` / `[chat]` / `[insight]` — default `[all]`
- hidden-count dim text next to filter: "201 hidden" (Nudge + legacy)
- no slash command; UI-only control

## loading / empty / error states

- **first load**: spinner briefly; fallback "no messages yet" if session has zero
- **scrollback exhausted**: subtle "beginning of session" marker
- **bridge disconnect**: non-modal warning strip at top of chat ("reconnecting…"); chat input greys but not disabled — if user types + submits, message queues and sends on reconnect

## phase-1b polish deferrals

- rich markdown rendering (code highlight, tables, images)
- emoji reactions
- pinned messages
- saved / starred messages
- rich link unfurls
- typing indicators
- search within chat
