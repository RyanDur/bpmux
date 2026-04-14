use crate::session::{Message, MessageType};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;
use tui_textarea::TextArea;

pub(super) const TRUNCATE_AFTER_LINES: usize = 10;
const DM_FLASH_DURATION: std::time::Duration = std::time::Duration::from_millis(750);

fn short_peer_name(peer: &str) -> &str {
    peer.split(':').next().unwrap_or(peer)
}

/// Owner is excluded from channel names — they're implicit in every channel.
pub fn channel_name(msg: &crate::session::Message, owner: &str) -> String {
    // #191: if the message has a topic, use the topic name as the
    // channel display tag. This makes topic messages show as
    // "[teaching]" instead of "[broadcast]" in the chat pane.
    if let Some(topic) = &msg.topic {
        return topic.clone();
    }
    match &msg.to {
        None => "broadcast".to_string(),
        Some(recipients) if recipients.iter().any(|r| r == "all") => "broadcast".to_string(),
        Some(recipients) => {
            let sender_short = msg.from.split(':').next().unwrap_or(&msg.from);
            let mut names: Vec<&str> = recipients.iter().map(|s| s.as_str()).collect();
            if !names.contains(&sender_short) {
                names.push(sender_short);
            }
            names.retain(|n| *n != owner);
            names.sort();
            if names.is_empty() {
                sender_short.to_string()
            } else if names.len() == 1 {
                names[0].to_string()
            } else {
                names.join("+")
            }
        }
    }
}

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

/// Board view state — extracted from App.
/// Command palette state — extracted from App.
pub struct PaletteState {
    pub show: bool,
    pub input: String,
    pub selected: usize,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            show: false,
            input: String::new(),
            selected: 0,
        }
    }
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

/// Scroll state — extracted from App.
/// Layout positions — set by the renderer each frame, read by
/// mouse click handler and status bar. Pure render state.
#[derive(Default)]
pub struct LayoutState {
    pub sidebar_area: Option<(u16, u16, u16, u16)>,
    pub menu_click_cell: Option<(u16, u16)>,
    pub status_bar_area: Option<(u16, u16, u16, u16)>,
    pub chat_area: Option<(u16, u16, u16, u16)>,
    pub help_hint_col: Option<u16>,
}

pub struct ScrollState {
    pub offset: usize,
    pub max: usize,
    pub at_boundary: bool,
    pub unread_count: usize,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            offset: 0,
            max: 0,
            at_boundary: false,
            unread_count: 0,
        }
    }
}

pub struct BoardState {
    pub show: bool,
    pub stories: Vec<super::board::BoardStory>,
    pub selected: usize,
    pub detail: Option<usize>,
}

impl Default for BoardState {
    fn default() -> Self {
        Self {
            show: false,
            stories: Vec::new(),
            selected: 0,
            detail: None,
        }
    }
}

pub struct App {
    // ── Session identity ──
    pub session_dir: Option<PathBuf>,
    pub peer_name: String,
    pub session_id: String,

    // ── Message state ──
    pub messages: Vec<Message>,
    /// Cached classification results to avoid re-classifying every
    /// message on every render frame. Keyed by message index.
    /// Rebuilt when messages are reloaded (lag recovery).
    pub class_cache: Vec<Option<crate::session::MessageClass>>,
    /// Cached indices of visible messages. Rebuilt when filter_dirty is set.
    pub filtered_indices: Vec<usize>,
    pub cached_hidden_count: usize,
    pub filter_dirty: bool,
    pub expanded_messages: HashSet<String>,
    pub truncation_indicators: Vec<(u16, String)>,

    // ── Input ──
    pub input: String,
    pub cursor_pos: usize,
    pub textarea: TextArea<'static>,
    pub input_history: Vec<String>,
    pub history_index: Option<usize>,
    pub popup_selected: Option<usize>,

    // ── Scroll ──
    pub scroll: ScrollState,

    // ── Channel view ──
    pub active_channels: HashSet<String>,
    pub current_view: Option<String>,
    pub send_target: Option<String>,
    pub channel_unread: HashMap<String, usize>,

    // ── Peers ──
    pub peer_activity: HashMap<String, (String, Instant)>,
    pub dm_unread_from: Vec<String>,
    pub dm_flash_until: Option<Instant>,

    // ── Sidebar ──
    pub sidebar: SidebarState,

    // ── Search ──
    pub search_results: Vec<crate::session::SearchResult>,
    pub search_selected: Option<usize>,

    // ── Project view ──
    pub project_state: Option<super::board::ProjectState>,
    pub show_project_view: bool,
    pub project_view_content: Option<String>,

    // ── Tokens view (#179) ──
    /// Whether the `/tokens` overlay is currently rendered. Dismissed
    /// by any key press, same UX as the `?` help overlay.
    pub show_tokens_view: bool,
    /// Snapshot of the token log rows loaded when `/tokens` was
    /// invoked. Kept on `App` (not re-read on every render) so the
    /// overlay stays stable across frames until dismissed.
    pub tokens_view_rows: Vec<crate::session::tokens::TokenRow>,

    // ── Usage statistics (#195) ──
    /// Per-feature invocation counts: command name → (count, last_used).
    /// Incremented at dispatch time in submit_message() and palette.
    pub feature_usage: HashMap<String, (usize, chrono::DateTime<chrono::Utc>)>,

    // ── Focus mode part A (#181) ──
    /// When `Some(roles)`, the chat view filter narrows to messages
    /// from the listed roles. Transient per-session (resets on
    /// restart) — per story decision. `None` = no filter.
    /// See `commands_focus::in_focus` for the override rules
    /// (own messages, system messages, addressee=my_role bypass).
    pub focused_roles: Option<Vec<String>>,

    // ── Command palette (#192) ──
    pub palette: PaletteState,

    // ── Topics (#191) ──
    pub topic: TopicState,

    // ── Board view (#193) ──
    pub board: BoardState,

    // ── CI status (#151) ──
    pub(super) ci_state: super::ci::CiState,

    // ── Display toggles ──
    pub show_timestamps: bool,
    pub show_system_messages: bool,
    pub show_help: bool,
    pub tui_focused: bool,

    // ── #161: classifier-driven view filter ──
    /// When `false` (default), the chat pane hides `MessageClass::Routine`
    /// messages and bpmux auto-nudges to reduce visual noise. Toggle with
    /// `/all` or Ctrl+A. Persisted via `UiState`.
    pub show_all_classes: bool,
    /// Count of messages suppressed by the class filter on the most
    /// recent render. Set by `chat::render_chat`, read by
    /// `status_bar::render` for the `N hidden` counter.
    pub hidden_class_count: usize,

    // ── Layout positions (set by renderer) ──
    pub layout: LayoutState,

    // ── Lifecycle ──
    pub should_quit: bool,
    pub should_reload: bool,
    pub p2p_command: Option<crate::transport::peer::P2pCommand>,
}

impl App {
    pub fn new(peer_name: String, session_id: String, messages: Vec<Message>) -> Self {
        let mut active_channels: HashSet<String> = HashSet::new();
        let peer_activity: HashMap<String, (String, Instant)> = HashMap::new();
        for msg in &messages {
            if msg.is_chat() {
                active_channels.insert(channel_name(msg, &peer_name));
            }
        }
        Self {
            session_dir: None,
            class_cache: Vec::new(),
            filtered_indices: Vec::new(),
            cached_hidden_count: 0,
            filter_dirty: true,
            messages,
            input: String::new(),
            cursor_pos: 0,
            textarea: {
                let mut ta = TextArea::default();
                ta.set_hard_tab_indent(false);
                ta.set_wrap_mode(tui_textarea::WrapMode::Word);
                ta
            },
            scroll: ScrollState::default(),
            peer_name,
            session_id,
            should_quit: false,
            show_timestamps: false,
            input_history: Vec::new(),
            history_index: None,
            peer_activity,
            show_system_messages: false,
            expanded_messages: HashSet::new(),
            dm_unread_from: Vec::new(),
            dm_flash_until: None,
            active_channels,
            current_view: None,
            popup_selected: None,
            show_help: false,
            sidebar: SidebarState::default(),
            channel_unread: HashMap::new(),
            send_target: None,
            tui_focused: true,
            search_results: Vec::new(),
            search_selected: None,
            layout: LayoutState::default(),
            truncation_indicators: Vec::new(),
            p2p_command: None,
            project_state: None,
            show_project_view: false,
            project_view_content: None,
            show_tokens_view: false,
            tokens_view_rows: Vec::new(),
            feature_usage: HashMap::new(),
            focused_roles: None,
            palette: PaletteState::default(),
            topic: TopicState::default(),
            board: BoardState::default(),
            should_reload: false,
            ci_state: super::ci::CiState::Unknown,
            show_all_classes: false,
            hidden_class_count: 0,
        }
    }

    /// Sync input/cursor_pos from the textarea widget.
    pub(super) fn sync_input_from_textarea(&mut self) {
        self.input = self.textarea.lines().join("");
        let (row, col) = self.textarea.cursor();
        let line_offset: usize = self.textarea.lines()[..row].iter().map(|l| l.len()).sum();
        self.cursor_pos = line_offset + col;
    }

    /// Push text into the textarea (replacing current content).
    pub(super) fn set_textarea_content(&mut self, text: &str) {
        let mut ta = TextArea::from([text]);
        ta.set_hard_tab_indent(false);
        ta.set_wrap_mode(tui_textarea::WrapMode::Word);
        ta.move_cursor(tui_textarea::CursorMove::End);
        self.textarea = ta;
        self.sync_input_from_textarea();
    }

    /// Single entry point for adding a message to the display list.
    /// All code paths that add messages should go through this so
    /// the class_cache stays in sync.
    /// Single entry point for adding a message to the display list.
    /// Maintains class_cache and marks the filter index dirty.
    pub(super) fn push_message(&mut self, msg: Message) {
        let class = if msg.is_chat() {
            Some(crate::session::classify(&msg, &self.peer_name))
        } else {
            None
        };
        self.class_cache.push(class);
        self.messages.push(msg);
        self.filter_dirty = true;
    }

    pub(super) fn system_message(&mut self, text: String) {
        self.push_message(Message {
            from: "bpmux".to_string(),
            text,
            ts: Utc::now(),
            message_type: MessageType::Chat,
            to: None,
            addressee: None,
            explicit_privacy: false,
            multi_depth: None,
            topic: None,
        });
    }

    /// Rebuild the filtered index if dirty. Called at the start of
    /// render_chat to avoid per-frame full scans.
    pub fn rebuild_filtered_if_dirty(&mut self) {
        // Also rebuild if indices are empty but messages exist
        // (covers test setups and initial state).
        if !self.filter_dirty && !self.filtered_indices.is_empty() {
            return;
        }
        if self.messages.is_empty() {
            return;
        }
        self.filter_dirty = false;
        self.filtered_indices.clear();
        self.cached_hidden_count = 0;
        for (idx, m) in self.messages.iter().enumerate() {
            if !m.is_chat() {
                if self.show_system_messages {
                    self.filtered_indices.push(idx);
                }
                continue;
            }
            let in_channel = match &self.current_view {
                None => true,
                Some(view) if view == "broadcast" => {
                    channel_name(m, &self.peer_name) == "broadcast"
                }
                Some(view) => super::markdown::message_matches_channel(m, view, &self.peer_name),
            };
            if !in_channel {
                continue;
            }
            if !self.show_all_classes {
                // Replicate should_hide_by_class logic exactly:
                // 1. Nudge type → hide
                // 2. bpmux sender → hide only 📝-prefixed
                // 3. Otherwise → hide if classified as Routine
                let hide = if m.message_type == crate::session::MessageType::Nudge {
                    true
                } else {
                    let sender = m.from.split(':').next().unwrap_or(&m.from);
                    if sender == "bpmux" {
                        m.text.starts_with('\u{1F4DD}')
                    } else {
                        let class = self
                            .class_cache
                            .get(idx)
                            .and_then(|c| c.as_ref())
                            .copied()
                            .unwrap_or_else(|| crate::session::classify(m, &self.peer_name));
                        matches!(class, crate::session::MessageClass::Routine)
                    }
                };
                if hide {
                    self.cached_hidden_count += 1;
                    continue;
                }
            }
            if !super::commands_focus::in_focus(m, self.focused_roles.as_deref(), &self.peer_name) {
                continue;
            }
            if let Some(msg_topic) = &m.topic {
                if *msg_topic != self.topic.active {
                    continue;
                }
            }
            self.filtered_indices.push(idx);
        }
    }

    /// Mark the filtered index as needing rebuild.
    pub fn mark_filters_dirty(&mut self) {
        self.filter_dirty = true;
    }

    /// #195: record a feature/command invocation for usage statistics.
    /// Updates in-memory state and persists to `~/.bpmux/usage.json`.
    pub fn record_usage(&mut self, feature: &str) {
        let entry = self
            .feature_usage
            .entry(feature.to_string())
            .or_insert((0, chrono::Utc::now()));
        entry.0 += 1;
        entry.1 = chrono::Utc::now();

        // Persist to disk so agents can query via MCP.
        if let Ok(home) = dirs::home_dir().ok_or(()) {
            let path = home.join(".bpmux").join("usage.json");
            let serializable: std::collections::HashMap<String, (usize, String)> = self
                .feature_usage
                .iter()
                .map(|(k, (c, t))| (k.clone(), (*c, t.to_rfc3339())))
                .collect();
            let _ = std::fs::create_dir_all(path.parent().unwrap_or(&path));
            let _ = serde_json::to_string_pretty(&serializable)
                .ok()
                .and_then(|json| std::fs::write(&path, json).ok());
        }
    }

    /// #192: return the filtered list of palette actions matching
    /// the current palette_input. Fuzzy match: the input chars must
    /// appear in order in the action name, but not necessarily
    /// adjacent.
    pub fn palette_matches(&self) -> Vec<(&'static str, &'static str)> {
        use super::input::SLASH_COMMANDS;
        let query = self.palette.input.to_lowercase();
        if query.is_empty() {
            return SLASH_COMMANDS
                .iter()
                .map(|(name, short, _)| (*name, *short))
                .collect();
        }
        SLASH_COMMANDS
            .iter()
            .filter_map(|(name, short, _)| {
                let haystack = name.to_lowercase();
                let mut hay_iter = haystack.chars();
                let matches = query.chars().all(|qc| hay_iter.any(|hc| hc == qc));
                if matches { Some((*name, *short)) } else { None }
            })
            .collect()
    }

    pub fn scroll_up(&mut self, lines: usize) {
        let before = self.scroll.offset;
        self.scroll.offset = self
            .scroll
            .offset
            .saturating_add(lines)
            .min(self.scroll.max);
        if self.scroll.offset != before {
            self.scroll.at_boundary = false;
        }
    }

    pub fn scroll_down(&mut self, lines: usize) {
        let before = self.scroll.offset;
        self.scroll.offset = self.scroll.offset.saturating_sub(lines);
        if self.scroll.offset != before {
            self.scroll.at_boundary = false;
        }
        if self.scroll.offset == 0 {
            self.scroll.unread_count = 0;
        }
    }

    pub fn toggle_last_truncated(&mut self) {
        let width = 78usize; // approximate inner width
        for msg in self.messages.iter().rev() {
            if !msg.is_chat() && !self.show_system_messages {
                continue;
            }
            let prefix_len = msg.from.split(':').next().unwrap_or(&msg.from).len() + 2;
            let full_len = prefix_len + msg.text.len();
            let wrapped = full_len.max(1).div_ceil(width.max(1));
            if wrapped > TRUNCATE_AFTER_LINES {
                let key = msg.ts.to_rfc3339();
                if self.expanded_messages.contains(&key) {
                    self.expanded_messages.remove(&key);
                } else {
                    self.expanded_messages.insert(key);
                }
                return;
            }
        }
    }

    fn rebuild_channels(&mut self) {
        let connected: std::collections::HashSet<&str> =
            self.peer_activity.keys().map(|s| s.as_str()).collect();
        self.active_channels = self
            .messages
            .iter()
            .filter(|m| m.is_chat())
            .map(|m| channel_name(m, &self.peer_name))
            .filter(|ch| {
                if ch == "broadcast" {
                    return true;
                }
                let has_connected_peer = ch.split('+').any(|name| connected.contains(name));
                let has_unread = self.channel_unread.get(ch).is_some_and(|&n| n > 0);
                has_connected_peer || has_unread
            })
            .collect();
    }

    pub fn update_activity(&mut self, peer: String, status: String) {
        self.peer_activity.insert(peer, (status, Instant::now()));
    }

    pub fn receive_message(&mut self, msg: Message) {
        let sender = short_peer_name(&msg.from).to_string();

        match msg.message_type {
            MessageType::Activity => {
                self.update_activity(sender, msg.text);
                return;
            }
            MessageType::System => {
                // #191: auto-register topics from creation notifications.
                // When a "Topic 'X' created" system message arrives,
                // add the topic to our list so /topic shows it.
                if msg.text.starts_with("Topic '")
                    && msg.text.contains("' created")
                    && let Some(start) = msg.text.find('\'')
                    && let Some(end) = msg.text[start + 1..].find('\'')
                {
                    let topic_name = msg.text[start + 1..start + 1 + end].to_string();
                    if !self.topic.topics.contains(&topic_name) {
                        self.topic.topics.push(topic_name);
                    }
                }
                if let Some(peer) = msg.text.strip_suffix(" joined") {
                    let short = short_peer_name(peer).to_string();
                    self.update_activity(short.clone(), "available".into());
                    self.active_channels.insert(short);
                    self.rebuild_channels();
                } else if let Some(peer) = msg.text.strip_suffix(" left") {
                    self.peer_activity.remove(short_peer_name(peer));
                    self.rebuild_channels();
                }
            }
            MessageType::Chat | MessageType::Insight | MessageType::Nudge => {
                // #191: auto-discover topics from incoming messages.
                // If a message has a topic we don't know about yet,
                // register it so /topic list shows it and the sidebar
                // (future) can display it.
                if let Some(topic) = &msg.topic {
                    if !self.topic.topics.contains(topic) {
                        self.topic.topics.push(topic.clone());
                    }
                    // Track unread per topic when not viewing that topic.
                    if *topic != self.topic.active {
                        *self.topic.unreads.entry(topic.clone()).or_insert(0) += 1;
                    }
                }
                let ch = channel_name(&msg, &self.peer_name);
                self.active_channels.insert(ch.clone());
                if self.current_view.as_ref().is_none_or(|v| v != &ch) {
                    *self.channel_unread.entry(ch).or_insert(0) += 1;
                }
                if let Some(rest) = msg.text.strip_prefix("/status ") {
                    self.update_activity(sender.clone(), rest.trim().to_string());
                }
                if let Some(recipients) = &msg.to {
                    let directed_at_me = recipients.iter().any(|r| r == &self.peer_name)
                        && !recipients.iter().all(|r| r == "all");
                    if directed_at_me && !self.dm_unread_from.contains(&sender) {
                        self.dm_unread_from.push(sender);
                        self.dm_flash_until = Some(Instant::now() + DM_FLASH_DURATION);
                    }
                }
            }
        }

        // #180: estimate the number of rendered lines this message
        // will occupy and compensate scroll_offset by that amount.
        // The render loop wraps text at ~78 chars (80 - 2 borders).
        // The prefix is approximately "[channel] sender: " = ~15 + name.
        // A separator line is added when the sender differs from the
        // previous message. This estimate is close enough to prevent
        // perceptible viewport drift — within ±1 line is fine.
        let was_at_bottom = self.scroll.offset == 0;
        let line_estimate = {
            let sender = msg.from.split(':').next().unwrap_or(&msg.from);
            let prefix_len = sender.len() + 15;
            const INNER_WIDTH: usize = 78;
            // Content width = inner width minus prefix/indent (both
            // first and continuation lines use the same indent).
            let content_width = INNER_WIDTH.saturating_sub(prefix_len).max(1);
            let wrapped = msg.text.len().max(1).div_ceil(content_width).max(1);
            // Add 1 for the blank separator line between different senders.
            // No separator for the first message or when sender matches.
            let separator = self
                .messages
                .last()
                .map(|m| {
                    let prev = m.from.split(':').next().unwrap_or(&m.from);
                    if prev != sender { 1 } else { 0 }
                })
                .unwrap_or(0);
            wrapped + separator
        };
        self.push_message(msg);
        if !was_at_bottom {
            self.scroll.offset = self.scroll.offset.saturating_add(line_estimate);
            self.scroll.unread_count += 1;
        }
    }
}
