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

pub struct App {
    // ── Session identity ──
    pub session_dir: Option<PathBuf>,
    pub peer_name: String,
    pub session_id: String,

    // ── Message state ──
    pub messages: Vec<Message>,
    pub expanded_messages: HashSet<String>,
    pub truncation_indicators: Vec<(u16, String)>,

    // ── Input ──
    pub input: String,
    pub cursor_pos: usize,
    pub textarea: TextArea<'static>,
    pub input_history: Vec<String>,
    pub history_index: Option<usize>,
    pub popup_selected: Option<usize>,

    // ── Scroll (#180: anchor model) ──
    pub scroll_offset: usize,
    pub max_scroll: usize,
    pub at_boundary: bool,
    pub unread_count: usize,
    /// #180: when the user is scrolled up, this holds the absolute
    /// scroll-from-top value (the row the Paragraph is scrolled to).
    /// `None` = at bottom (auto-follow). `Some(n)` = viewport is
    /// anchored at row n from the top of the content. New messages
    /// don't change this value, so the viewport doesn't drift.
    pub scroll_anchor: Option<usize>,

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
    pub sidebar_open: bool,
    pub sidebar_focused: bool,
    pub sidebar_selected: usize,
    pub sidebar_filter: String,
    pub project_expanded: bool,
    pub direct_expanded: bool,
    pub groups_expanded: bool,
    pub system_expanded: bool,

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

    // ── Focus mode part A (#181) ──
    /// When `Some(roles)`, the chat view filter narrows to messages
    /// from the listed roles. Transient per-session (resets on
    /// restart) — per story decision. `None` = no filter.
    /// See `commands_focus::in_focus` for the override rules
    /// (own messages, system messages, addressee=my_role bypass).
    pub focused_roles: Option<Vec<String>>,

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
    pub sidebar_area: Option<(u16, u16, u16, u16)>,

    /// (#183) Coordinates of the ≡ menu-toggle glyph rendered at the
    /// top-left of the chat area when the sidebar is closed. Set by
    /// the renderer each frame; read by the mouse click handler to
    /// detect "click on the menu icon" without relying on the old
    /// "col < 3 on any row" heuristic which produced false positives
    /// on any upper-left click. `None` when the sidebar is open.
    pub menu_click_cell: Option<(u16, u16)>,
    pub status_bar_area: Option<(u16, u16, u16, u16)>,
    pub chat_area: Option<(u16, u16, u16, u16)>,
    pub help_hint_col: Option<u16>,

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
            messages,
            input: String::new(),
            cursor_pos: 0,
            textarea: {
                let mut ta = TextArea::default();
                ta.set_hard_tab_indent(false);
                ta.set_wrap_mode(tui_textarea::WrapMode::Word);
                ta
            },
            scroll_offset: 0,
            max_scroll: 0,
            at_boundary: false,
            scroll_anchor: None,
            peer_name,
            session_id,
            should_quit: false,
            show_timestamps: false,
            unread_count: 0,
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
            sidebar_open: false,
            sidebar_focused: false,
            sidebar_selected: 0,
            channel_unread: HashMap::new(),
            send_target: None,
            tui_focused: true,
            project_expanded: true,
            sidebar_filter: String::new(),
            search_results: Vec::new(),
            search_selected: None,
            sidebar_area: None,
            menu_click_cell: None,
            status_bar_area: None,
            help_hint_col: None,
            truncation_indicators: Vec::new(),
            p2p_command: None,
            project_state: None,
            show_project_view: false,
            project_view_content: None,
            show_tokens_view: false,
            tokens_view_rows: Vec::new(),
            focused_roles: None,
            chat_area: None,
            should_reload: false,
            direct_expanded: true,
            groups_expanded: true,
            system_expanded: false,
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

    pub(super) fn system_message(&mut self, text: String) {
        self.messages.push(Message {
            from: "bpmux".to_string(),
            text,
            ts: Utc::now(),
            message_type: MessageType::Chat,
            to: None,
            addressee: None,
            explicit_privacy: false,
            multi_depth: None,
        });
    }

    pub fn scroll_up(&mut self, lines: usize) {
        let before = self.scroll_offset;
        self.scroll_offset = self
            .scroll_offset
            .saturating_add(lines)
            .min(self.max_scroll);
        if self.scroll_offset != before {
            self.at_boundary = false;
        }
        // #180: set the anchor from the current render position.
        // scroll = max_scroll - scroll_offset gives the absolute
        // row-from-top that the Paragraph is scrolled to.
        self.scroll_anchor = Some(self.max_scroll.saturating_sub(self.scroll_offset));
    }

    pub fn scroll_down(&mut self, lines: usize) {
        let before = self.scroll_offset;
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        if self.scroll_offset != before {
            self.at_boundary = false;
        }
        if self.scroll_offset == 0 {
            // Back at bottom — clear anchor, resume auto-follow.
            self.scroll_anchor = None;
            self.unread_count = 0;
        } else {
            self.scroll_anchor = Some(self.max_scroll.saturating_sub(self.scroll_offset));
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

        // #180 (anchor model): when scrolled up (anchor is Some),
        // the viewport is pinned to an absolute row. Don't touch
        // scroll_offset — the render loop will use scroll_anchor
        // directly and new messages don't affect it. Just increment
        // unread_count so the indicator shows.
        //
        // When at bottom (anchor is None), scroll_offset stays 0
        // and the render loop shows the latest messages as usual.
        self.messages.push(msg);
        if self.scroll_anchor.is_some() {
            self.unread_count += 1;
        }
    }
}
