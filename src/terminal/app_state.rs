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

/// Topic state — extracted from App.
pub struct TopicState {
    /// Known topics in this session. "general" always present.
    pub topics: Vec<String>,
    /// Which topic the user is currently viewing/typing into.
    pub active: String,
    /// Per-topic unread count for sidebar display.
    pub unreads: HashMap<String, usize>,
    /// Per-topic participant lists.
    pub participants: HashMap<String, Vec<String>>,
}

impl Default for TopicState {
    fn default() -> Self {
        Self {
            topics: vec!["general".to_string()],
            active: "general".to_string(),
            unreads: HashMap::new(),
            participants: HashMap::new(),
        }
    }
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

/// Message state — extracted from App.
pub struct MessageState {
    pub messages: Vec<Message>,
    pub class_cache: Vec<Option<crate::session::MessageClass>>,
    pub filtered_indices: Vec<usize>,
    pub cached_hidden_count: usize,
    pub filter_dirty: bool,
    pub expanded_messages: HashSet<String>,
    pub truncation_indicators: Vec<(u16, String)>,
}

impl MessageState {
    pub(super) fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            class_cache: Vec::new(),
            filtered_indices: Vec::new(),
            cached_hidden_count: 0,
            filter_dirty: true,
            expanded_messages: HashSet::new(),
            truncation_indicators: Vec::new(),
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
/// invited (mid-thread add is a follow-on story).
#[derive(Debug, Clone)]
pub struct Thread {
    pub id: String,
    pub name: String,
    pub participants: Vec<String>,
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
}
