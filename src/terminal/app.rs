use crate::session::{Message, MessageType};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

const TRUNCATE_AFTER_LINES: usize = 10;
const DM_FLASH_DURATION: std::time::Duration = std::time::Duration::from_millis(750);

/// Strips the transport suffix from a peer name: "dev:claude-code" → "dev".
fn short_peer_name(peer: &str) -> &str {
    peer.split(':').next().unwrap_or(peer)
}

/// Compute the display name for a message's channel.
/// Returns "broadcast" for `None` / `["all"]`, "a↔b" for 2-person DMs,
/// "a+b+c" for groups. The sender is included in directed channels.
/// Compute the channel name for a message. The `owner` (TUI user) is excluded
/// from the name — they're implicit in every channel, like a manager who can
/// see all Slack rooms. This prevents the view/send mismatch where sending
/// into a channel creates a different channel name because the owner got added.
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
            // Remove the TUI owner — they're a constant, not a participant to name
            names.retain(|n| *n != owner);
            names.sort();
            if names.is_empty() {
                // DM to/from the owner only — show the other party
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
    pub connected_peers: usize,
    pub unread_count: usize,
    pub input_history: Vec<String>,
    pub history_index: Option<usize>,
    pub peer_activity: HashMap<String, (String, Instant)>,
    pub show_system_messages: bool,
    /// Expanded (un-truncated) messages, keyed by timestamp string for stability.
    pub expanded_messages: HashSet<String>,
    /// Senders of unacknowledged directed messages (DMs). Cleared on user input.
    pub dm_unread_from: Vec<String>,
    /// When set, the status bar flashes yellow until this instant passes.
    pub dm_flash_until: Option<Instant>,
    /// Distinct channel names seen this session (e.g. "broadcast", "dev↔human", "dev+human+pm").
    pub active_channels: HashSet<String>,
    /// Current view filter. `None` = all, `Some("broadcast")` = broadcast only,
    /// `Some("a↔b")` or `Some("a+b+c")` = specific directed channel.
    pub current_view: Option<String>,
    /// Selected index in the command autocomplete popup. `None` = nothing selected.
    pub popup_selected: Option<usize>,
    /// Pinned theory items. Max 5, persisted in session state.
    pub pins: Vec<String>,
    /// Whether the help overlay is visible.
    pub show_help: bool,
    /// Whether the sidebar tree view is open.
    pub sidebar_open: bool,
    /// Whether keyboard focus is on the sidebar (vs input).
    pub sidebar_focused: bool,
    /// Index of selected item in the sidebar channel list.
    pub sidebar_selected: usize,
    /// Unread message count per channel.
    pub channel_unread: HashMap<String, usize>,
    /// Send target set by sidebar selection. `None` = broadcast.
    /// When set, outgoing messages are directed to this channel's participants.
    pub send_target: Option<String>,
    /// Whether the terminal window has focus. Used to gate OS notifications.
    /// Defaults to true (assume focused — no false noise).
    pub tui_focused: bool,
    /// Whether the project tree node is expanded (showing channels).
    pub project_expanded: bool,
    /// Layout regions for mouse click detection (updated each render pass).
    pub sidebar_area: Option<(u16, u16, u16, u16)>,
    /// Status bar area: (x, y, width, height).
    pub status_bar_area: Option<(u16, u16, u16, u16)>,
    /// Column where "? help" starts in the status bar.
    pub help_hint_col: Option<u16>,
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
            connected_peers: 0,
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
            pins: Vec::new(),
            show_help: false,
            sidebar_open: false,
            sidebar_focused: false,
            sidebar_selected: 0,
            channel_unread: HashMap::new(),
            send_target: None,
            tui_focused: true,
            project_expanded: true,
            sidebar_area: None,
            status_bar_area: None,
            help_hint_col: None,
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
