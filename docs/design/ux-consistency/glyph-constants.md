# UX spec: glyph constants

*"Good looks like" target for centralizing all unicode glyphs in icons.rs.*

## Current state

`icons.rs` defines 3 constants. The codebase uses 15+ distinct glyphs
hardcoded as raw unicode escapes or literal characters across ui.rs,
sidebar.rs, status_bar.rs, commands_search.rs, commands_topic.rs,
board.rs, ci.rs, and chat.rs.

## Target state

Every user-visible glyph is a named constant in `icons.rs`. No raw
`\u{xxxx}` or literal `▸` anywhere else in the codebase.

## Complete glyph inventory

```rust
// icons.rs — single source of truth for all TUI glyphs

// Navigation & structure
pub(super) const EXPAND: &str = "\u{25B8}";      // ▸
pub(super) const COLLAPSE: &str = "\u{25BE}";    // ▾
pub(super) const MENU: &str = "\u{2261}";         // ≡
pub(super) const OPTION_KEY: &str = "\u{2325}";   // ⌥
pub(super) const UP_ARROW: &str = "\u{2191}";     // ↑
pub(super) const DOWN_ARROW: &str = "\u{2193}";   // ↓
pub(super) const RIGHT_ARROW: &str = "\u{2192}";  // →

// Status indicators
pub(super) const ACTIVE: &str = "\u{25CF}";       // ●
pub(super) const INACTIVE: &str = "\u{25CB}";     // ○
pub(super) const CHECK: &str = "\u{2713}";        // ✓
pub(super) const CROSS: &str = "\u{2717}";        // ✗
pub(super) const HIGHLIGHT: &str = "\u{25C0}";    // ◀

// Communication
pub(super) const ENVELOPE: &str = "\u{2709}";     // ✉

// Typography
pub(super) const EM_DASH: &str = "\u{2014}";      // —
pub(super) const ELLIPSIS: &str = "\u{2026}";     // …
```

## Migration

Each file that uses a raw glyph replaces it with the constant:

```
BEFORE                                  AFTER
────────────────────────────────────────────────────────────
ui.rs:626       "\u{25B8} "             icons::EXPAND
ui.rs:152       "\u{2325}"              icons::OPTION_KEY
sidebar.rs      "▸ " (6 occurrences)    icons::EXPAND
sidebar.rs      "▾ " (2 occurrences)    icons::COLLAPSE
sidebar.rs:76   "● " (literal)          icons::ACTIVE
status_bar.rs   "\u{25CF}"              icons::ACTIVE
status_bar.rs   "\u{2709}"              icons::ENVELOPE
status_bar.rs   "\u{2191}\u{2193}"      icons::UP_ARROW, icons::DOWN_ARROW
ci.rs           "\u{2713}"              icons::CHECK
ci.rs           "\u{2717}"              icons::CROSS
ci.rs           "\u{25CB}"              icons::INACTIVE
ci.rs           "\u{2014}"              icons::EM_DASH
ci.rs           "\u{2026}"              icons::ELLIPSIS
ci.rs           "\u{2192}"              icons::RIGHT_ARROW
board.rs        "● " (literal)          icons::ACTIVE
board.rs        "○ " (literal)          icons::INACTIVE
topic.rs        " ●" (literal)          icons::ACTIVE
search.rs       " ◀" (literal)          icons::HIGHLIGHT
```

## Note on emoji glyphs

The codebase also uses emoji (e.g., in chat.rs, ci.rs) which are
multi-codepoint sequences. These are a separate category from single-
codepoint glyphs and may warrant their own constants, but are out of
scope for this spec.

## Why this matters

1. **One place to change** — if we switch glyph sets (e.g., for a
   terminal that doesn't render unicode), one file changes
2. **Searchable** — `grep icons::EXPAND` finds every expand arrow.
   `grep "▸"` misses the `\u{25B8}` form and vice versa
3. **Reviewable** — designer can audit the glyph palette in one file
   instead of hunting across the codebase
