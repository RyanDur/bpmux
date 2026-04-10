# Interactivity Design Patterns for bpmux TUI

*Designer research — 2026-04-08. Directed by Ryan.*

## IntelliJ Reference Patterns

**Tool window strip model:** Icons on thin strips along edges. Click toggles open/close. Alt+number toggles specific window. Same shortcut again hides it. Escape always returns to editor.

**Focus hierarchy:** Escape → editor. F12 → last tool window. Alt+N → specific window. Shift+Escape → hide + return to editor.

**Type-to-filter:** When a list has focus, typing immediately filters. No search mode needed.

## TUI Conventions (lazygit, vim, tmux, k9s, Midnight Commander)

**Numbered panel jumping (lazygit):** Panels 1-5, number key jumps focus. Constant-time access. Spatial layout never changes.

**Click-to-focus, double-click-to-act (Midnight Commander):** Click selects, double-click opens. Tab switches panels.

**Mouse as optional accelerator (vim):** Every mouse action has a keyboard equivalent.

**F-key action bar (htop):** Persistent action bar at bottom showing available commands.

**Status bar updates contextually:** Show different shortcuts based on which panel has focus.

## Direct Manipulation (Shneiderman)

Three principles applied to TUI:

1. **Continuous representation:** Channel list, unread counts, current channel name always visible.
2. **Rapid, reversible, incremental:** Channel switching instant, previous-channel key for undo.
3. **Physical actions replace syntax:** Click/Enter on channel name instead of typing `/join`.

## Terminal Mouse Support

**Safe default:** Mode 1002 (button-event tracking) + SGR 1006. Click and scroll work everywhere.

**Do NOT use hover (mode 1003):** Terminal.app doesn't support it. Floods events. Minimal UX value in 80x24.

**Respect Shift+click:** Let terminal handle native text selection for copy.

**Compatibility:** Click and scroll are universal across iTerm2, Terminal.app, Alacritty, kitty, Windows Terminal, WezTerm.

## Focus Management

**Three-panel focus ring for bpmux:**
- Tab/Shift+Tab: sidebar → chat → input (cycle)
- Number keys: 1=sidebar, 2=chat, 3=input (direct jump)
- Escape: always returns to input bar (home position for a chat app)
- Mouse click: focuses clicked panel

**Visual indicators (use 2+ for redundancy):**
- Border color change (focused = accent color, unfocused = dim)
- Border style (bold/double-line vs single-line)
- Title highlight

**Input bar is home.** In a chat app, the input bar is the primary workspace. Escape returns there. Default focus on startup.

## Recommendations for bpmux

1. **Keyboard-first, mouse-supported.** Design for keyboard, add mouse as accelerator.
2. **Escape is sacred.** Always de-escalates: closes popups, returns to input, cancels actions.
3. **Spatial stability.** Layout doesn't change when focus moves. Only borders/colors change.
4. **Contextual status bar.** Show 4-6 relevant shortcuts for current focus context.
5. **Toggle, don't navigate.** Sidebar toggle with one key (⌥1), not menu navigation.
6. **Type-to-filter in channel list.** When sidebar focused, typing filters channels.
7. **Test with tmux.** Mouse events and key sequences behave differently in multiplexers.

## What bpmux Already Does Well

- ⌥1 sidebar toggle (IntelliJ pattern)
- Mouse scroll in chat
- Click to select channels
- Help overlay with ? key
- Status bar with contextual info

## What to Add

- Click sidebar header to close (IntelliJ tool window close)
- Focus indicators (border color/style change for focused panel)
- Contextual status bar (update shortcuts based on focused panel)
- Type-to-filter in channel list
- Previous-channel key (Alt+Left or similar)
- Escape from chat scrolling returns to input

## Sources

- IntelliJ Tool Windows Documentation
- Lazygit keybindings
- Shneiderman 1983 — Direct Manipulation
- Xterm Control Sequences
- Ratatui mouse capture docs
- Midnight Commander, k9s, htop interaction patterns
