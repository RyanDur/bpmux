//! Sub-state structs extracted from `App` (God Object reduction).
//!
//! Each struct groups a cohesive set of fields that were formerly
//! loose on `App`. They live here to keep `app.rs` focused on the
//! `App` type definition and its `impl` methods.

use crate::session::Message;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;
use tui_textarea::TextArea;

/// Sidebar navigation state — extracted from App to reduce the God Object.
/// Methods live in sidebar.rs. Mutated by sidebar key handlers;
/// read by the sidebar renderer and event dispatch.
pub struct SidebarState {
    pub open: bool,
    pub focused: bool,
    pub selected: usize,
    pub filter: String,
    pub project_expanded: bool,
    pub direct_expanded: bool,
    pub groups_expanded: bool,
    pub system_expanded: bool,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            open: false,
            focused: false,
            selected: 0,
            filter: String::new(),
            project_expanded: true,
            direct_expanded: true,
            groups_expanded: true,
            system_expanded: false,
        }
    }
}

/// Which modal overlay is active. Only one can be active at a time.
/// Replaces the implicit priority chain of boolean flags that
/// required reading the entire event_dispatch to understand ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal chat input — no overlay active.
    #[default]
    Chat,
    /// Help overlay (? key).
    Help,
    /// Tokens + usage overlay (/tokens).
    Tokens,
    /// Command palette (Ctrl+K).
    Palette,
    /// Project panel (/board, header click).
    Project,
    /// Full-screen board detail.
    Board,
    /// Sidebar navigation (Alt+1).
    Sidebar,
}

#[derive(Default)]
pub struct PaletteState {
    pub input: String,
    pub selected: usize,
}

/// Search state — extracted from App.
#[derive(Default)]
pub struct SearchState {
    pub results: Vec<crate::session::SearchResult>,
    pub selected: Option<usize>,
}

/// Project view state — extracted from App. Visibility is driven
/// by `App::input_mode == InputMode::Project`; this struct holds
/// only the data the panel renders when it IS visible.
#[derive(Default)]
pub struct ProjectViewState {
    pub state: Option<super::board::ProjectState>,
    pub content: Option<String>,
    /// Cached DORA metrics; populated by the periodic
    /// project-state tick (src/terminal/event.rs). None until the
    /// first fetch completes — renders as "insufficient data" then.
    pub dora: Option<super::board::DoraMetrics>,
}

/// Tokens overlay state — extracted from App. Visibility is driven
/// by `App::input_mode == InputMode::Tokens`.
#[derive(Default)]
pub struct TokensViewState {
    pub rows: Vec<crate::session::tokens::TokenRow>,
}

/// Depth at which a multi-depth message is currently rendered in the
/// chat pane (#285). `Summary` is the shallowest (one-line); `RoleDefault`
/// is the role-tuned variant from `MultiDepthPayload.role_default`; `Deep`
/// is the full detail. Click on a multi-depth message cycles through the
/// levels: Summary → RoleDefault → Deep → Summary.
///
/// Distinct from `MultiDepthPayload` (wire schema) — this is per-viewer
/// TUI state, stored ephemerally in `ui-state.json` alongside
/// `expanded_messages`. Post-compact collapses to the role-default
/// initial state by design (TUI-layer re-render, not work-state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DepthLevel {
    Summary,
    RoleDefault,
    Deep,
}

impl DepthLevel {
    /// Default depth for a role seeing a multi-depth message before
    /// any click. AC #285: dev sees role_default (full stage detail);
    /// pm/designer see summary. Other roles fall back to Summary.
    pub fn initial_for_role(role: &str) -> Self {
        if role == "dev" {
            Self::RoleDefault
        } else {
            Self::Summary
        }
    }

    /// Advance one step in the cycle: Summary → RoleDefault → Deep → Summary.
    pub fn next(self) -> Self {
        match self {
            Self::Summary => Self::RoleDefault,
            Self::RoleDefault => Self::Deep,
            Self::Deep => Self::Summary,
        }
    }
}

/// Message state — extracted from App.
pub struct MessageState {
    pub messages: Vec<Message>,
    pub class_cache: Vec<Option<crate::session::MessageClass>>,
    pub filtered_indices: Vec<usize>,
    pub cached_hidden_count: usize,
    pub filter_dirty: bool,
    /// Truncation expansion state (#110) AND multi-depth cycle state
    /// (#285) keyed by `msg.ts.to_rfc3339()`. For long-text messages,
    /// presence in the map (at any depth) means "expanded past the
    /// truncation threshold." For multi-depth messages, the stored
    /// `DepthLevel` is the current cycle position; absence means
    /// "use `DepthLevel::initial_for_role(role)`." The two uses never
    /// collide because a message is either long-text-truncated OR
    /// multi-depth-rendered (AC #285: msg.text is suppressed when
    /// multi_depth is present).
    pub expanded_messages: HashMap<String, DepthLevel>,
    pub truncation_indicators: Vec<(u16, String)>,
    /// Multi-depth click-zone indicators (#285) — `(screen_row, msg_key)`
    /// so the mouse handler can route a click on a depth affordance to
    /// the right message. Separate from `truncation_indicators` so
    /// layer routing stays unambiguous.
    pub depth_indicators: Vec<(u16, String)>,
}

impl MessageState {
    pub(super) fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            class_cache: Vec::new(),
            filtered_indices: Vec::new(),
            cached_hidden_count: 0,
            filter_dirty: true,
            expanded_messages: HashMap::new(),
            truncation_indicators: Vec::new(),
            depth_indicators: Vec::new(),
        }
    }
}

/// Channel view state — extracted from App.
#[derive(Default)]
pub struct ChannelState {
    pub active: HashSet<String>,
    pub current_view: Option<String>,
    pub send_target: Option<String>,
    pub unread: HashMap<String, usize>,
}

/// Peer presence state — extracted from App.
#[derive(Default)]
pub struct PeerState {
    pub activity: HashMap<String, (String, Instant)>,
    pub dm_unread_from: Vec<String>,
    pub dm_flash_until: Option<Instant>,
}

/// Display toggle state — extracted from App. `show_help` moved
/// out: help-overlay visibility is driven by
/// `App::input_mode == InputMode::Help`.
pub struct DisplayState {
    pub show_timestamps: bool,
    pub show_system_messages: bool,
    pub tui_focused: bool,
    pub show_all_classes: bool,
    pub hidden_class_count: usize,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            show_timestamps: false,
            show_system_messages: false,
            tui_focused: true,
            show_all_classes: false,
            hidden_class_count: 0,
        }
    }
}

#[derive(Default)]
pub struct LayoutState {
    pub sidebar_area: Option<(u16, u16, u16, u16)>,
    pub menu_click_cell: Option<(u16, u16)>,
    pub status_bar_area: Option<(u16, u16, u16, u16)>,
    pub chat_area: Option<(u16, u16, u16, u16)>,
    pub help_hint_col: Option<u16>,
    /// Column range (start..=end, inclusive) of the CI indicator in
    /// the status bar. Mouse handler reads this to route clicks to
    /// `open_in_browser` on the GitHub Actions URL. `None` when the
    /// indicator isn't rendered (Unknown state or no space).
    pub ci_indicator_col_range: Option<(u16, u16)>,
}

#[derive(Default)]
pub struct ScrollState {
    pub offset: usize,
    pub max: usize,
    pub at_boundary: bool,
    pub unread_count: usize,
}

/// Board state — extracted from App. The full-screen board is driven
/// by `App::input_mode == InputMode::Board`; the side-by-side board
/// panel (#318) is driven by `panel_open` and coexists with chat.
#[derive(Default)]
pub struct BoardState {
    pub stories: Vec<super::board::BoardStory>,
    pub selected: usize,
    pub detail: Option<usize>,
    /// Right-panel board visibility (#318). When true + terminal is
    /// wide enough, the layout splits main_area into chat (left) +
    /// board panel (right). Toggled via Alt+2, parallel to the
    /// sidebar's Alt+1. Independent of InputMode::Board (which is
    /// the full-screen overlay).
    pub panel_open: bool,
}

/// Session identity — frozen after construction. These fields
/// are set once in App::new() and must not change during the session.
pub struct SessionIdentity {
    pub dir: Option<PathBuf>,
    pub name: String,
    pub id: String,
}

/// Text-input editor state — input buffer, cursor, history, and the
/// completion popup selection that hangs off the editor. Grouped
/// from previously-loose App fields so adding a new editor concern
/// (e.g. selection range) lands in one place.
pub struct EditorState {
    pub input: String,
    pub cursor_pos: usize,
    pub textarea: TextArea<'static>,
    pub input_history: Vec<String>,
    pub history_index: Option<usize>,
    /// Index into the slash-completion popup; `None` when the
    /// popup isn't accepting a selection.
    pub popup_selected: Option<usize>,
}

impl Default for EditorState {
    fn default() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_hard_tab_indent(false);
        textarea.set_wrap_mode(tui_textarea::WrapMode::Word);
        Self {
            input: String::new(),
            cursor_pos: 0,
            textarea,
            input_history: Vec::new(),
            history_index: None,
            popup_selected: None,
        }
    }
}

/// Process-lifecycle flags — quit/reload requests, the queued P2P
/// command awaiting service, and the pending restart prompt. These
/// are control-flow signals the event loop polls each tick.
#[derive(Default)]
pub struct LifecycleState {
    pub should_quit: bool,
    pub should_reload: bool,
    pub p2p_command: Option<crate::transport::peer::P2pCommand>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod depth_level_tests {
    use super::*;

    #[test]
    fn dev_starts_at_role_default() {
        // AC #285: "dev sees payload.role_default['dev'] (full stage detail)"
        assert_eq!(DepthLevel::initial_for_role("dev"), DepthLevel::RoleDefault);
    }

    #[test]
    fn pm_starts_at_summary() {
        // AC #285: "pm / designer see payload.summary (one-liner)"
        assert_eq!(DepthLevel::initial_for_role("pm"), DepthLevel::Summary);
    }

    #[test]
    fn designer_starts_at_summary() {
        assert_eq!(
            DepthLevel::initial_for_role("designer"),
            DepthLevel::Summary
        );
    }

    #[test]
    fn unknown_role_falls_back_to_summary() {
        // Shape: "viewer's role comes from identity_name" — any role we
        // don't special-case defaults to the shallowest view (Summary).
        assert_eq!(DepthLevel::initial_for_role("ryan"), DepthLevel::Summary);
        assert_eq!(DepthLevel::initial_for_role(""), DepthLevel::Summary);
        assert_eq!(DepthLevel::initial_for_role("unknown"), DepthLevel::Summary);
    }

    #[test]
    fn cycle_summary_to_role_default() {
        // AC #285: "Click cycles depth for that message: Summary →
        // RoleDefault → Deep → Summary". Each step independently.
        assert_eq!(DepthLevel::Summary.next(), DepthLevel::RoleDefault);
    }

    #[test]
    fn cycle_role_default_to_deep() {
        assert_eq!(DepthLevel::RoleDefault.next(), DepthLevel::Deep);
    }

    #[test]
    fn cycle_deep_back_to_summary() {
        // Deep cycles back to Summary, closing the loop.
        assert_eq!(DepthLevel::Deep.next(), DepthLevel::Summary);
    }

    #[test]
    fn three_cycles_returns_to_start() {
        // Full loop invariant: three .next() calls bring us back
        // regardless of starting position.
        for start in [
            DepthLevel::Summary,
            DepthLevel::RoleDefault,
            DepthLevel::Deep,
        ] {
            assert_eq!(start.next().next().next(), start);
        }
    }

    #[test]
    fn depth_level_serde_round_trips() {
        // UiState persists DepthLevel in ui-state.json; confirm all
        // three variants round-trip through serde_json.
        for variant in [
            DepthLevel::Summary,
            DepthLevel::RoleDefault,
            DepthLevel::Deep,
        ] {
            let json = serde_json::to_string(&variant).expect("serde_json::to_string");
            let back: DepthLevel =
                serde_json::from_str(&json).expect("serde_json::from_str round-trip");
            assert_eq!(back, variant);
        }
    }
}
