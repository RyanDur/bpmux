# UX spec: glyph constants

*"Good looks like" target for centralizing all unicode glyphs in icons.rs.*

## Current state

`icons.rs` defines 3 constants. The codebase uses 8+ distinct glyphs
hardcoded as raw unicode escapes or literal characters across ui.rs,
sidebar.rs, status_bar.rs, commands_search.rs, commands_topic.rs,
board.rs, and ci.rs.

## Target state

Every user-visible glyph is a named constant in `icons.rs`. No raw
`\u{xxxx}` or literal `▸` anywhere else in the codebase.

## Complete glyph inventory

```rust
// icons.rs — single source of truth for all TUI glyphs

pub(super) const EXPAND: &str = "\u{25B8}";     // ▸
pub(super) const COLLAPSE: &str = "\u{25BE}";   // ▾
pub(super) const MENU: &str = "\u{2261}";        // ≡
pub(super) const OPTION_KEY: &str = "\u{2325}";  // ⌥
pub(super) const ACTIVE: &str = "\u{25CF}";      // ●
pub(super) const INACTIVE: &str = "\u{25CB}";    // ○
pub(super) const CHECK: &str = "\u{2713}";       // ✓
pub(super) const CROSS: &str = "\u{2717}";       // ✗
pub(super) const HIGHLIGHT: &str = "\u{25C0}";   // ◀
```

## Migration

Each file that uses a raw glyph replaces it with the constant:

```
BEFORE                              AFTER
──────────────────────────────────────────────────────
ui.rs:626     "\u{25B8} "           icons::EXPAND
ui.rs:152     "\u{2325}"            icons::OPTION_KEY
sidebar.rs    "▸ " (6 occurrences)  icons::EXPAND
sidebar.rs    "▾ " (2 occurrences)  icons::COLLAPSE
status_bar.rs "\u{25CF}"            icons::ACTIVE
ci.rs         "\u{2713}"            icons::CHECK
ci.rs         "\u{2717}"            icons::CROSS
ci.rs         "\u{25CB}"            icons::INACTIVE
board.rs      "● "                  icons::ACTIVE
topic.rs      " ●"                  icons::ACTIVE
search.rs     " ◀"                  icons::HIGHLIGHT
```

## Why this matters

1. **One place to change** — if we switch glyph sets (e.g., for a
   terminal that doesn't render unicode), one file changes
2. **Searchable** — `grep icons::EXPAND` finds every expand arrow.
   `grep "▸"` misses the `\u{25B8}` form and vice versa
3. **Reviewable** — designer can audit the glyph palette in one file
   instead of hunting across the codebase
