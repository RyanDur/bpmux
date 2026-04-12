# Command palette — replacing slash commands

*Designer — 2026-04-12. Replaces slash commands as the primary interaction model.*

## Problem

18+ slash commands require memorization and typing. not discoverable, not fast, not contextual. ryan: "can we come up with something better than slash commands?"

## Solution

Three interaction layers that compose:
1. **Command palette** (primary) — fuzzy-searchable popup for all actions
2. **Keybinds** (fast path) — direct keys for top 5-6 actions
3. **Sidebar** (navigation) — clickable items for topic/channel switching

Slash commands become an escape hatch for scripting/automation, not the primary path.

## Mockups

- [Palette open](palette-open.md) — user opens palette and types to filter
- [Palette selected](palette-selected.md) — user selects an action
- [Keybind hints](keybind-hints.md) — status bar shows available shortcuts

## Story

#191 context coherence is the primary story. command palette may be its own story or a slice.
