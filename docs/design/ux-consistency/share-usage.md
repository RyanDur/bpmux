# UX spec: /share usage text

*"Good looks like" target for the /share help output.*

## Current behavior

```
Usage: /share <scope> [message]. Forms: `/share all` (propagate last message) or `/share dev "text"` (send fresh). Scope: role name(s) or 'all'.
```

144 characters on one line. Wraps at column 80. Hard to scan.

## Target behavior

```
/share all           share last message with everyone
/share dev           share last message with dev
/share dev "text"    send fresh message to dev
```

## Rules

1. **Three lines, one form per line** — example on left, description on right
2. **No "Usage:" prefix** — the user asked for help, they know it's usage
3. **Align columns** — command left-justified, description right-justified with consistent padding
4. **Under 80 characters per line** — no wrapping

## Mockup

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ryan: can you share that with dev?                                          │
│                                                                              │
│  /share all           share last message with everyone                       │
│  /share dev           share last message with dev                            │
│  /share dev "text"    send fresh message to dev                              │
│                                                                              │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
┌ designer > ──────────────────────────────────────────────────────────────────┐
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
 general                                                            Enter=send
```

Each form is its own line. Scannable at a glance. Fits in 80 columns
with room to spare.
