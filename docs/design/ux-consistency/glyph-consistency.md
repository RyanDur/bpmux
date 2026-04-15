# UX spec: glyph consistency

*"Good looks like" target for consistent visual symbols across the TUI.*

## Problem

The same concept uses different glyphs in different places, and some
glyphs appear without clear meaning. A user scanning the screen should
be able to learn "● means active" once and trust it everywhere.

## The glyph palette

Each visual concept has ONE glyph. The same glyph means the same thing
everywhere it appears.

```
CONCEPT          GLYPH    WHERE IT APPEARS
──────────────────────────────────────────────────────────────
expand/drill     ▸        sidebar sections, command palette items
collapse         ▾        sidebar expanded sections
active/selected  ●        sidebar active channel, topic active marker,
                          board active stories, peer status dots
inactive         ○        board backlog, CI pending
success          ✓        CI passing
failure          ✗        CI failing
current match    ◀        search result highlight
menu             ≡        menu affordance
DM indicator     ✉        status bar DM notification
```

## Rules

1. **One glyph per concept** — don't use ● for "active channel" in the
   sidebar and a different symbol for "active story" on the board
2. **Every glyph has a meaning** — if a user asks "what does ▸ mean?"
   the answer is always "you can expand this"
3. **Consistent spacing** — glyph + single space before label (`▸ Stories`),
   or single space + glyph after label (`design ●`)

## Current inconsistencies

- Sidebar uses literal `▸` characters; other places use unicode escapes
  for the same glyph — they render identically but this makes it harder
  for dev to keep them consistent. Dev decides how to solve that.
- Board "Icebox" section uses `◌` (dotted circle) which isn't in the
  palette above. Should it be `○` (same as inactive/backlog) or is
  icebox a distinct concept that deserves its own glyph?

## What this spec does NOT cover

- Which Rust constants to create or how to organize them — dev's call
- Emoji (colored circles for CI, memo icon for notes) — separate
  category with different rendering concerns
- Glyph rendering on different terminals — if we need a fallback set,
  that's a separate spec
