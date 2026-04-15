# UX spec: /focus role validation

*"Good looks like" target for the /focus command with unknown role names.*

## Current behavior

```
designer > /focus dve
```

Silently accepts the typo. Messages disappear. No feedback. User thinks
the system is broken.

## Target behavior

```
designer > /focus dve
Unknown role "dve". Known: dev, pm, designer, ryan.

designer > /focus dev
Focus: dev.

designer > /focus dev,deisgner
Unknown role "deisgner". Known: dev, pm, designer, ryan.
Focus: dev.
```

## Rules

1. **Validate each role** against `peer_activity` keys (known peers seen this session)
2. **Warn per unknown role** — don't silently drop it
3. **Apply the valid ones** — if the user typed `/focus dev,deisgner`, still apply `dev`
4. **Show what stuck** — confirm which roles are now focused after validation

## Edge cases

```
/focus dev,dev           → Focus: dev.              (deduplicate silently)
/focus                   → Focus cleared.           (no argument = clear)
/focus asdf              → Unknown role "asdf". Known: dev, pm, designer, ryan.
                           Focus unchanged.
/focus designer          → Focus: designer.         (focusing on self is valid —
                                                     filters to only your own messages)
/focus ryan              → Focus: ryan.             (human is a valid peer)
```

## Data source

Validate against `peer_activity` keys in `app.rs`. This includes all
peers seen during the current session (human and agent). If `ryan` has
sent a message, `ryan` is a valid focus target. If no peers have been
seen yet, the "Known:" list is empty and ALL role names are unknown —
that's correct behavior (you can't focus on someone who hasn't appeared).

## Mockup

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ryan: how's the board?                                                      │
│                                                                              │
│  dev: shipped the panel view.                                                │
│                                                                              │
│  Unknown role "dve". Known: dev, pm, designer, ryan.                         │
│                                                                              │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
┌ designer > ──────────────────────────────────────────────────────────────────┐
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
 general | focus unchanged                                          Enter=send
```

The warning appears as a system message in the chat stream. Status bar
reflects the actual focus state.
