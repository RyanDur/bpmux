# Command palette: keybind hints in status bar

*Status bar shows direct keybinds for the top actions alongside the palette shortcut.*

## Default state (in chat, not scrolled)

```
 teaching protocol                  Ctrl+K=actions ⌥1=topics ?=help Enter=send
```

## Scrolled up (Esc context changes)

```
 teaching protocol | 3 new below           Ctrl+K=actions Esc=bottom Enter=send
```

## Focus mode active

```
 teaching protocol [focus: pm,designer]    Ctrl+K=actions ⌥1=topics Enter=send
```

## Visual notes

- `Ctrl+K=actions` is always visible — the entry point to the palette
- Other keybinds shown contextually (Esc=bottom only when scrolled up)
- Clickable items in DRACULA_CYAN (per #183 affordance spec)
- Non-clickable keybind labels in DRACULA_COMMENT (dim)
- Status bar adapts to state without cluttering — shows what's RELEVANT now

## The keybind vocabulary (top 6, always available)

| Key | Action | When shown in status bar |
|---|---|---|
| Ctrl+K | Open command palette | Always |
| ⌥1 | Toggle sidebar/topics | Always (when not in palette) |
| ? | Open help overlay | Always (when not in palette) |
| Esc | Context-dependent: close overlay / jump to bottom / clear input | When relevant |
| Enter | Send message | Always (in chat mode) |
| Tab | Switch to next topic with unreads | When topics have unreads |

Everything else lives in the command palette — discoverable by opening it.
