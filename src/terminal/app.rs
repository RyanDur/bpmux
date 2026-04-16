use crate::session::{Message, MessageType};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tui_textarea::TextArea;

pub use super::app_state::*;

pub(super) const TRUNCATE_AFTER_LINES: usize = 10;
/// Approximate chat pane inner width (80 terminal - 2 borders).
const CHAT_INNER_WIDTH: usize = 78;
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
        Some(recipients)
            if recipients
                .iter()
                .any(|r| r == crate::session::message::BROADCAST_ALL) =>
        {
            "broadcast".to_string()
        }
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
    pub identity: SessionIdentity,

    // ── Message state ──
    pub msg: MessageState,

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
    pub channels: ChannelState,

    // ── Peers ──
    pub peers: PeerState,

    // ── Sidebar ──
    pub sidebar: SidebarState,

    // ── Search ──
    pub search: SearchState,

    // ── Project view ──
    pub project: ProjectViewState,

    // ── Tokens view (#179) ──
    pub tokens: TokensViewState,

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

    // ── Board poller snapshot (#219) — background-refreshed state ──
    pub board_snapshot: std::sync::Arc<std::sync::Mutex<super::board_poller::BoardSnapshot>>,

    // ── Modal state ──
    pub input_mode: InputMode,

    // ── CI status (#151) ──
    pub(super) ci_state: super::ci::CiState,

    // ── Display toggles ──
    pub display: DisplayState,

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
            identity: SessionIdentity {
                dir: None,
                name: peer_name.clone(),
                id: session_id.clone(),
            },
            msg: MessageState::new(messages),
            input: String::new(),
            cursor_pos: 0,
            textarea: {
                let mut ta = TextArea::default();
                ta.set_hard_tab_indent(false);
                ta.set_wrap_mode(tui_textarea::WrapMode::Word);
                ta
            },
            scroll: ScrollState::default(),
            should_quit: false,
            display: DisplayState::default(),
            input_history: Vec::new(),
            history_index: None,
            peers: PeerState {
                activity: peer_activity,
                ..Default::default()
            },
            channels: ChannelState {
                active: active_channels,
                current_view: None,
                send_target: None,
                unread: HashMap::new(),
            },
            popup_selected: None,
            sidebar: SidebarState::default(),
            search: SearchState::default(),
            layout: LayoutState::default(),
            p2p_command: None,
            project: ProjectViewState::default(),
            tokens: TokensViewState::default(),
            feature_usage: HashMap::new(),
            focused_roles: None,
            palette: PaletteState::default(),
            topic: TopicState::default(),
            board: BoardState::default(),
            board_snapshot: std::sync::Arc::new(std::sync::Mutex::new(
                super::board_poller::BoardSnapshot::default(),
            )),
            input_mode: InputMode::Chat,
            should_reload: false,
            ci_state: super::ci::CiState::Unknown,
        }
    }

    pub(super) fn sync_input_from_textarea(&mut self) {
        self.input = self.textarea.lines().join("");
        let (row, col) = self.textarea.cursor();
        let line_offset: usize = self.textarea.lines()[..row].iter().map(|l| l.len()).sum();
        self.cursor_pos = line_offset + col;
    }

    pub(super) fn set_textarea_content(&mut self, text: &str) {
        let mut ta = TextArea::from([text]);
        ta.set_hard_tab_indent(false);
        ta.set_wrap_mode(tui_textarea::WrapMode::Word);
        ta.move_cursor(tui_textarea::CursorMove::End);
        self.textarea = ta;
        self.sync_input_from_textarea();
    }

    pub fn from_identity(&self) -> String {
        format!("{}:human", self.identity.name)
    }

    /// Single entry point for adding a message. Maintains class_cache
    /// and marks the filter index dirty.
    pub(super) fn push_message(&mut self, msg: Message) {
        let class = if msg.is_chat() {
            Some(crate::session::classify(&msg, &self.identity.name))
        } else {
            None
        };
        self.msg.class_cache.push(class);
        self.msg.messages.push(msg);
        self.msg.filter_dirty = true;
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
        if !self.msg.filter_dirty && !self.msg.filtered_indices.is_empty() {
            return;
        }
        if self.msg.messages.is_empty() {
            return;
        }
        self.msg.filter_dirty = false;
        self.msg.filtered_indices.clear();
        self.msg.cached_hidden_count = 0;
        for (idx, m) in self.msg.messages.iter().enumerate() {
            if !m.is_chat() {
                if self.display.show_system_messages {
                    self.msg.filtered_indices.push(idx);
                }
                continue;
            }
            let in_channel = match &self.channels.current_view {
                None => true,
                Some(view) if view == "broadcast" => {
                    channel_name(m, &self.identity.name) == "broadcast"
                }
                Some(view) => {
                    super::markdown::message_matches_channel(m, view, &self.identity.name)
                }
            };
            if !in_channel {
                continue;
            }
            if !self.display.show_all_classes
                && super::chat::should_hide_by_class(m, &self.identity.name)
            {
                self.msg.cached_hidden_count += 1;
                continue;
            }
            if !super::commands::focus::in_focus(
                m,
                self.focused_roles.as_deref(),
                &self.identity.name,
            ) {
                continue;
            }
            if let Some(msg_topic) = &m.topic
                && *msg_topic != self.topic.active
            {
                continue;
            }
            self.msg.filtered_indices.push(idx);
        }
    }

    pub fn mark_filters_dirty(&mut self) {
        self.msg.filter_dirty = true;
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
    }

    /// Persist usage data to disk. Called at session end or on
    /// explicit save, not on every command invocation. Separates
    /// the I/O side effect from the in-memory counter update.
    pub fn save_usage_to_disk(&self) {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".bpmux").join("usage.json");
            let serializable: std::collections::HashMap<String, (usize, String)> = self
                .feature_usage
                .iter()
                .map(|(k, (c, t))| (k.clone(), (*c, t.to_rfc3339())))
                .collect();
            if let Err(e) = std::fs::create_dir_all(path.parent().unwrap_or(&path)) {
                crate::session::diag::log_diag(&format!("save_usage mkdir failed: {e}"));
                return;
            }
            match serde_json::to_string_pretty(&serializable) {
                Ok(json) => {
                    if let Err(e) = std::fs::write(&path, json) {
                        crate::session::diag::log_diag(&format!("save_usage write failed: {e}"));
                    }
                }
                Err(e) => {
                    crate::session::diag::log_diag(&format!("save_usage serialize failed: {e}"));
                }
            }
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
        let width = CHAT_INNER_WIDTH;
        for msg in self.msg.messages.iter().rev() {
            if !msg.is_chat() && !self.display.show_system_messages {
                continue;
            }
            let prefix_len = msg.from.split(':').next().unwrap_or(&msg.from).len() + 2;
            let full_len = prefix_len + msg.text.len();
            let wrapped = full_len.max(1).div_ceil(width.max(1));
            if wrapped > TRUNCATE_AFTER_LINES {
                let key = msg.ts.to_rfc3339();
                if self.msg.expanded_messages.contains(&key) {
                    self.msg.expanded_messages.remove(&key);
                } else {
                    self.msg.expanded_messages.insert(key);
                }
                return;
            }
        }
    }

    fn rebuild_channels(&mut self) {
        let connected: std::collections::HashSet<&str> =
            self.peers.activity.keys().map(|s| s.as_str()).collect();
        self.channels.active = self
            .msg
            .messages
            .iter()
            .filter(|m| m.is_chat())
            .map(|m| channel_name(m, &self.identity.name))
            .filter(|ch| {
                if ch == "broadcast" {
                    return true;
                }
                let has_connected_peer = ch.split('+').any(|name| connected.contains(name));
                let has_unread = self.channels.unread.get(ch).is_some_and(|&n| n > 0);
                has_connected_peer || has_unread
            })
            .collect();
    }

    pub fn update_activity(&mut self, peer: String, status: String) {
        self.peers.activity.insert(peer, (status, Instant::now()));
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
                    self.channels.active.insert(short);
                    self.rebuild_channels();
                } else if let Some(peer) = msg.text.strip_suffix(" left") {
                    self.peers.activity.remove(short_peer_name(peer));
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
                let ch = channel_name(&msg, &self.identity.name);
                self.channels.active.insert(ch.clone());
                if self.channels.current_view.as_ref().is_none_or(|v| v != &ch) {
                    *self.channels.unread.entry(ch).or_insert(0) += 1;
                }
                if let Some(rest) = msg.text.strip_prefix("/status ") {
                    self.update_activity(sender.clone(), rest.trim().to_string());
                }
                if let Some(recipients) = &msg.to {
                    let directed_at_me = recipients.iter().any(|r| r == &self.identity.name)
                        && !recipients
                            .iter()
                            .all(|r| r == crate::session::message::BROADCAST_ALL);
                    if directed_at_me && !self.peers.dm_unread_from.contains(&sender) {
                        self.peers.dm_unread_from.push(sender);
                        self.peers.dm_flash_until = Some(Instant::now() + DM_FLASH_DURATION);
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
            let content_width = CHAT_INNER_WIDTH.saturating_sub(prefix_len).max(1);
            let wrapped = msg.text.len().max(1).div_ceil(content_width).max(1);
            // Add 1 for the blank separator line between different senders.
            // No separator for the first message or when sender matches.
            let separator = self
                .msg
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
