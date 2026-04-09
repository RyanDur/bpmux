use crate::session::{Message, MessageType};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

const TRUNCATE_AFTER_LINES: usize = 10;
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
    pub session_dir: Option<PathBuf>,
    pub messages: Vec<Message>,
    pub input: String,
    pub cursor_pos: usize,
    pub scroll_offset: usize,
    pub max_scroll: usize,
    pub at_boundary: bool,
    pub peer_name: String,
    pub session_id: String,
    pub should_quit: bool,
    pub show_timestamps: bool,
    pub unread_count: usize,
    pub input_history: Vec<String>,
    pub history_index: Option<usize>,
    pub peer_activity: HashMap<String, (String, Instant)>,
    pub show_system_messages: bool,
    pub expanded_messages: HashSet<String>,
    pub dm_unread_from: Vec<String>,
    pub dm_flash_until: Option<Instant>,
    pub active_channels: HashSet<String>,
    pub current_view: Option<String>,
    pub popup_selected: Option<usize>,
    pub show_help: bool,
    pub sidebar_open: bool,
    pub sidebar_focused: bool,
    pub sidebar_selected: usize,
    pub channel_unread: HashMap<String, usize>,
    pub send_target: Option<String>,
    /// Defaults to true — assume focused so we don't fire notifications on startup.
    pub tui_focused: bool,
    pub project_expanded: bool,
    pub sidebar_filter: String,
    pub search_results: Vec<crate::session::SearchResult>,
    pub search_selected: Option<usize>,
    pub sidebar_area: Option<(u16, u16, u16, u16)>,
    pub status_bar_area: Option<(u16, u16, u16, u16)>,
    pub help_hint_col: Option<u16>,
    /// Screen Y positions of "[+N lines]" truncation indicators and their message keys.
    pub truncation_indicators: Vec<(u16, String)>,
    pub p2p_command: Option<crate::transport::peer::P2pCommand>,
    /// Live project state from GitHub issues.
    pub project_state: Option<super::board::ProjectState>,
    /// When true, the main area shows the project view instead of chat.
    pub show_project_view: bool,
    /// Chat area position for title bar click detection.
    pub chat_area: Option<(u16, u16, u16, u16)>,
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
            scroll_offset: 0,
            max_scroll: 0,
            at_boundary: false,
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
            status_bar_area: None,
            help_hint_col: None,
            truncation_indicators: Vec::new(),
            p2p_command: None,
            project_state: None,
            show_project_view: false,
            chat_area: None,
        }
    }

    pub(super) fn system_message(&mut self, text: String) {
        self.messages.push(Message {
            from: "bpmux".to_string(),
            text,
            ts: Utc::now(),
            message_type: MessageType::Chat,
            to: None,
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
    }

    pub fn scroll_down(&mut self, lines: usize) {
        let before = self.scroll_offset;
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        if self.scroll_offset != before {
            self.at_boundary = false;
        }
        if self.scroll_offset == 0 {
            self.unread_count = 0;
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
                    self.update_activity(short_peer_name(peer).to_string(), "available".into());
                } else if let Some(peer) = msg.text.strip_suffix(" left") {
                    self.peer_activity.remove(short_peer_name(peer));
                    self.rebuild_channels();
                }
            }
            MessageType::Chat => {
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

        let was_at_bottom = self.scroll_offset == 0;
        self.messages.push(msg);
        if !was_at_bottom {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
            self.unread_count += 1;
        }
    }
}
