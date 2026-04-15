//! Glyph vocabulary for the bpmux TUI (#183).
//!
//! Keeps icon strings in one place so the visual language is
//! consistent across every surface and changeable in one edit.
//! Designer's #183 spec pointed at IntelliJ and web conventions as
//! the pattern source — each constant here maps directly to a
//! borrowed convention:
//!
//! - `EXPAND` / `COLLAPSE` (▸/▾) match IntelliJ tree widgets and
//!   most file-tree viewers.
//! - `MENU` (≡) matches the hamburger-menu pattern from mobile and
//!   web. Used as the left-edge-zone trigger so the previously
//!   invisible click region gets a visible affordance.
//!
//! Using these consistently means a user who learns one glyph can
//! predict its meaning elsewhere.

/// `▸` — expand / reveal (collapsed state).
pub(super) const EXPAND: &str = "\u{25B8}";

/// `▾` — collapse / hide (expanded state).
pub(super) const COLLAPSE: &str = "\u{25BE}";

/// `≡` — menu / toggle (hamburger icon).
pub(super) const MENU: &str = "\u{2261}";

/// `●` — active / present (topic, peer, channel).
pub(super) const ACTIVE: &str = "\u{25CF}";

/// `✉` — DM indicator (direct message notification).
pub(super) const DM: &str = "\u{2709}";

/// `↓` — down arrow (new-message indicator).
pub(super) const DOWN: &str = "\u{2193}";
