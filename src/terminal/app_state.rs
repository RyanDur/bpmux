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
    pub threads_expanded: bool,
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
            threads_expanded: true,
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
    /// CI-events-only filter (#295). When ACTIVE, the chat pane
    /// renders only CI-transition broadcasts (Activity messages with
    /// the `🔴 CI` / `🟢 CI` prefix). Lets Ryan find CI events in a
    /// long scrollback without scrolling — click CI-events sidebar
    /// section to toggle. Persists across restart via `UiState` like
    /// `show_all_classes` does, so a filtered session-close doesn't
    /// surprise-restore to "all" on reopen.
    pub ci_filter_active: bool,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            show_timestamps: false,
            show_system_messages: false,
            tui_focused: true,
            show_all_classes: false,
            hidden_class_count: 0,
            ci_filter_active: false,
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
}

#[derive(Default)]
pub struct ScrollState {
    pub offset: usize,
    pub max: usize,
    pub at_boundary: bool,
    pub unread_count: usize,
}

/// Board state — extracted from App. Visibility is driven by
/// `App::input_mode == InputMode::Board`.
#[derive(Default)]
pub struct BoardState {
    pub stories: Vec<super::board::BoardStory>,
    pub selected: usize,
    pub detail: Option<usize>,
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

/// One named conversation thread (#268). `id` is the unique handle
/// messages carry via `Message::thread_id`; `name` is the
/// human-readable label; `participants` are the roles initially
/// invited. `last_activity` and `unread` are per-thread tracking
/// for the sidebar re-entry affordance (#275).
#[derive(Debug, Clone)]
pub struct Thread {
    pub id: String,
    pub name: String,
    pub participants: Vec<String>,
    /// Most-recent activity timestamp — drives sidebar ordering
    /// (freshest at top). Set to the thread's creation time on
    /// register and bumped every time a thread-tagged message
    /// lands. `None` pre-#275 data paths (test setup).
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    /// Unread count for this thread, independent of the broadcast
    /// unread counter. Incremented when a thread-tagged message
    /// arrives AND the viewer isn't currently in that thread.
    /// Reset on re-entry.
    pub unread: usize,
}

/// Thread state for the TUI viewer. `known` is the local cache of
/// threads the bridge has told us about; `current` is the viewer's
/// active thread (None = broadcast view). `broadcast_unread_while_threaded`
/// is the peripheral counter that keeps cross-thread activity
/// visible while Ryan is focused inside a thread.
#[derive(Default)]
pub struct ThreadState {
    pub known: HashMap<String, Thread>,
    pub current: Option<String>,
    pub broadcast_unread_while_threaded: usize,
    /// Pending suggest_leave targeting this viewer (#273). Populated
    /// when a marker arrives with target_role matching self; cleared
    /// when the viewer leaves the thread (Esc) or dismisses (any other
    /// key). `None` = no banner showing.
    pub pending_suggestion: Option<PendingSuggestion>,
    /// Pending pull_into_thread invitation targeting this viewer
    /// (#274). Populated when a marker arrives with target_role
    /// matching self; cleared when the viewer accepts (via
    /// join_thread) or dismisses (any key). `None` = no banner.
    pub pending_invitation: Option<PendingInvitation>,
}

/// A suggest_leave banner waiting for the viewer's response (#273).
#[derive(Debug, Clone, PartialEq)]
pub struct PendingSuggestion {
    pub thread_name: String,
    pub caller: String,
    pub reason: String,
}

/// A pull_into_thread invitation banner (#274).
#[derive(Debug, Clone, PartialEq)]
pub struct PendingInvitation {
    pub thread_name: String,
    pub caller: String,
    pub reason: String,
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
