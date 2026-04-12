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
//! - `CLOSE` (×) matches tab close buttons everywhere.
//! - `MENU` (≡) matches the hamburger-menu pattern from mobile and
//!   web. Used as the left-edge-zone trigger so the previously
//!   invisible click region gets a visible affordance.
//! - `MORE` (⋮) matches material design "more actions" overflow.
//! - `BREADCRUMB` (›) matches IntelliJ breadcrumbs + web breadcrumb
//!   separators.
//!
//! Using these consistently means a user who learns one glyph can
//! predict its meaning elsewhere.

/// `▸` — expand / reveal (collapsed state).
pub(super) const EXPAND: &str = "\u{25B8}";

/// `▾` — collapse / hide (expanded state).
pub(super) const COLLAPSE: &str = "\u{25BE}";

/// `×` — close / dismiss (tab close button convention).
#[allow(dead_code)]
pub(super) const CLOSE: &str = "\u{00D7}";

/// `≡` — menu / toggle (hamburger icon).
pub(super) const MENU: &str = "\u{2261}";

/// `⋮` — more / overflow (material design).
#[allow(dead_code)]
pub(super) const MORE: &str = "\u{22EE}";

/// `›` — breadcrumb separator (IntelliJ + web convention).
pub(super) const BREADCRUMB: &str = "\u{203A}";
