use crate::session::Message;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

const MAX_HISTORY: usize = 50;

/// Registered slash commands with brief descriptions for the popup.
pub const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/to", "send to recipients"),
    ("/view", "filter view by channel"),
    ("/pin", "pin a decision"),
    ("/pins", "list pinned items"),
    ("/unpin", "remove a pin by number"),
    ("/board", "show project board"),
    ("/status", "declare your status"),
    ("/help", "show this help"),
    ("/who", "list participants"),
    ("/debug", "toggle system messages"),
    ("/quit", "exit bpmux"),
];

/// Compute the display name for a message's channel.
/// Returns "broadcast" for `None` / `["all"]`, "a↔b" for 2-person DMs,
/// "a+b+c" for groups. The sender is included in directed channels.
pub fn channel_name(msg: &crate::session::Message) -> String {
    match &msg.to {
        None => "broadcast".to_string(),
        Some(recipients) if recipients.iter().any(|r| r == "all") => "broadcast".to_string(),
        Some(recipients) => {
            let sender_short = msg.from.split(':').next().unwrap_or(&msg.from);
            let mut names: Vec<&str> = recipients.iter().map(|s| s.as_str()).collect();
            if !names.contains(&sender_short) {
                names.push(sender_short);
            }
            names.sort();
            if names.len() == 2 {
                format!("{}\u{2194}{}", names[0], names[1])
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
}

impl App {
    pub fn new(peer_name: String, session_id: String, messages: Vec<Message>) -> Self {
        let mut active_channels: HashSet<String> = HashSet::new();
        for msg in &messages {
            if msg.is_chat() {
                active_channels.insert(channel_name(msg));
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
            peer_activity: HashMap::new(),
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
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.popup_selected = None;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.remove(prev);
            self.cursor_pos = prev;
            self.popup_selected = None;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.input[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos = self.input[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.input.len());
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_pos = self.input.len();
    }

    pub fn kill_to_end(&mut self) {
        self.input.truncate(self.cursor_pos);
    }

    pub fn delete_word_backward(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let before = &self.input[..self.cursor_pos];
        let trimmed = before.trim_end();
        let word_start = trimmed
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        self.input.drain(word_start..self.cursor_pos);
        self.cursor_pos = word_start;
    }

    pub fn move_word_left(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let before = &self.input[..self.cursor_pos];
        let trimmed = before.trim_end();
        self.cursor_pos = trimmed
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
    }

    pub fn move_word_right(&mut self) {
        if self.cursor_pos >= self.input.len() {
            return;
        }
        let after = &self.input[self.cursor_pos..];
        let skip_word = after
            .find(|c: char| c.is_whitespace())
            .unwrap_or(after.len());
        let skip_space = after[skip_word..]
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(after.len() - skip_word);
        self.cursor_pos += skip_word + skip_space;
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
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
            if wrapped > 10 {
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
        if msg.message_type.as_deref() == Some("activity") {
            self.update_activity(msg.from, msg.text);
            return;
        }
        // Auto-set "available" when a peer joins, remove when they leave
        if msg.message_type.as_deref() == Some("system") {
            if let Some(peer) = msg.text.strip_suffix(" joined") {
                self.update_activity(peer.to_string(), "available".to_string());
            } else if let Some(peer) = msg.text.strip_suffix(" left") {
                self.peer_activity.remove(peer);
            }
        }
        // Track active channels and per-channel unread counts
        if msg.is_chat() {
            let ch = channel_name(&msg);
            self.active_channels.insert(ch.clone());
            // Increment unread if this channel isn't the current view
            let is_current = match &self.current_view {
                None => false, // "all" view — no specific channel to clear
                Some(v) => v == &ch,
            };
            if !is_current {
                *self.channel_unread.entry(ch).or_insert(0) += 1;
            }
        }
        // Track peer presence and self-declared status
        if msg.is_chat() {
            let peer = msg.from.clone();
            if let Some(rest) = msg.text.strip_prefix("/status ") {
                // Self-declared status: `/status blocked reason` or `/status working on X`
                self.update_activity(peer, rest.trim().to_string());
            } else {
                // Regular chat = proof of connection, but only set "connected"
                // if no richer status exists yet
                if !self.peer_activity.contains_key(&peer) {
                    self.update_activity(peer, "connected".to_string());
                } else {
                    // Refresh timestamp without overwriting self-declared status
                    let status = self.peer_activity[&peer].0.clone();
                    self.update_activity(peer, status);
                }
            }
        }
        // Track DM notifications for visual flash + badge
        if let Some(recipients) = &msg.to {
            let is_directed_at_me = recipients.iter().any(|r| r == &self.peer_name)
                && !recipients.iter().all(|r| r == "all");
            if is_directed_at_me {
                let sender = msg.from.split(':').next().unwrap_or(&msg.from).to_string();
                if !self.dm_unread_from.contains(&sender) {
                    self.dm_unread_from.push(sender);
                }
                self.dm_flash_until = Some(Instant::now() + std::time::Duration::from_millis(750));
            }
        }
        let was_at_bottom = self.scroll_offset == 0;
        self.messages.push(msg);
        if !was_at_bottom {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
            self.unread_count += 1;
        }
    }

    pub fn submit_message(&mut self) -> Option<Message> {
        self.dm_unread_from.clear();
        self.dm_flash_until = None;
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return None;
        }
        if text == "/help" {
            let help = format!(
                "Commands:\n  /to <who> <msg> — send to specific recipients (e.g. /to dev,pm hello)\n  /view [channel] — filter view (broadcast, all, dev\u{2194}human, etc.)\n  /pin <text>     — pin a decision (max 5)\n  /pins           — list pinned items\n  /unpin <n>      — remove pin by number\n  /board          — show project board\n  /status blocked [reason] — declare yourself blocked\n  /help     — show this help\n  /who      — list participants\n  /debug    — toggle system messages\n  /quit     — exit bpmux\n\nKeybindings:\n  ?         — help overlay\n  Ctrl+B    — toggle sidebar\n  Enter     — send message\n  Esc       — clear input\n  Ctrl+C    — clear input (empty: quit)\n  Ctrl+U/D  — scroll up/down\n  Ctrl+T    — toggle timestamps\n  PageUp/Dn — scroll\n\nSession: {}",
                self.session_id
            );
            let msg = Message {
                from: "bpmux".to_string(),
                text: help,
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text == "/who" {
            let mut participants = vec![format!("{} (you)", self.peer_name)];
            for (peer, (status, _)) in &self.peer_activity {
                let short = peer.split(':').next().unwrap_or(peer);
                participants.push(format!("{} [{}]", short, status));
            }
            if self.connected_peers > 0 {
                participants.push(format!("{} connected", self.connected_peers));
            }
            let msg = Message {
                from: "bpmux".to_string(),
                text: format!("Participants:\n  {}", participants.join("\n  ")),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text == "/quit" {
            self.should_quit = true;
            return None;
        }
        if text == "/debug" {
            self.show_system_messages = !self.show_system_messages;
            let state = if self.show_system_messages {
                "on"
            } else {
                "off"
            };
            let msg = Message {
                from: "bpmux".to_string(),
                text: format!(
                    "Debug mode {state}. System messages are now {}.",
                    if self.show_system_messages {
                        "visible"
                    } else {
                        "hidden"
                    }
                ),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text.starts_with("/status ") {
            // Send as a regular message — the convention is parsed by receivers
            let msg = Message {
                from: format!("{}:human", self.peer_name),
                text,
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg.clone());
            self.clear_input();
            self.scroll_offset = 0;
            self.unread_count = 0;
            return Some(msg);
        }
        if let Some(rest) = text.strip_prefix("/to ") {
            let rest = rest.trim_start();
            if let Some((recipients_str, body)) = rest.split_once(' ') {
                let body = body.trim().to_string();
                if body.is_empty() {
                    let msg = Message {
                        from: "bpmux".to_string(),
                        text: "Usage: /to <recipients> <message>. Example: /to dev,pm hello"
                            .to_string(),
                        ts: Utc::now(),
                        message_type: None,
                        to: None,
                    };
                    self.messages.push(msg);
                    self.clear_input();
                    return None;
                }
                let recipients: Vec<String> = recipients_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if recipients.is_empty() {
                    let msg = Message {
                        from: "bpmux".to_string(),
                        text: "Usage: /to <recipients> <message>. Example: /to dev,pm hello"
                            .to_string(),
                        ts: Utc::now(),
                        message_type: None,
                        to: None,
                    };
                    self.messages.push(msg);
                    self.clear_input();
                    return None;
                }
                let msg = Message {
                    from: format!("{}:human", self.peer_name),
                    text: body,
                    ts: Utc::now(),
                    message_type: None,
                    to: Some(recipients),
                };
                self.messages.push(msg.clone());
                self.clear_input();
                self.scroll_offset = 0;
                self.unread_count = 0;
                if self.input_history.len() >= MAX_HISTORY {
                    self.input_history.remove(0);
                }
                self.input_history.push(self.input.clone());
                self.history_index = None;
                return Some(msg);
            }
            // `/to <something>` with no body
            let msg = Message {
                from: "bpmux".to_string(),
                text: "Usage: /to <recipients> <message>. Example: /to dev,pm hello".to_string(),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text == "/to" {
            let msg = Message {
                from: "bpmux".to_string(),
                text: "Usage: /to <recipients> <message>. Example: /to dev,pm hello".to_string(),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text == "/view" {
            // List available channels
            let mut channel_names: Vec<&str> =
                self.active_channels.iter().map(|s| s.as_str()).collect();
            channel_names.sort();
            let list = if channel_names.is_empty() {
                "No channels yet.".to_string()
            } else {
                format!("Available channels: {}", channel_names.join(", "))
            };
            let msg = Message {
                from: "bpmux".to_string(),
                text: list,
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if let Some(arg) = text.strip_prefix("/view ") {
            let arg = arg.trim();
            if arg == "all" {
                self.current_view = None;
                self.scroll_offset = 0;
                let msg = Message {
                    from: "bpmux".to_string(),
                    text: "View set to all channels.".to_string(),
                    ts: Utc::now(),
                    message_type: None,
                    to: None,
                };
                self.messages.push(msg);
                self.clear_input();
                return None;
            }
            if arg == "broadcast" {
                self.current_view = Some("broadcast".to_string());
                self.scroll_offset = 0;
                let msg = Message {
                    from: "bpmux".to_string(),
                    text: "View set to broadcast.".to_string(),
                    ts: Utc::now(),
                    message_type: None,
                    to: None,
                };
                self.messages.push(msg);
                self.clear_input();
                return None;
            }
            // Check if arg matches a known channel
            if self.active_channels.contains(arg) {
                self.current_view = Some(arg.to_string());
                self.scroll_offset = 0;
                let msg = Message {
                    from: "bpmux".to_string(),
                    text: format!("View set to {arg}."),
                    ts: Utc::now(),
                    message_type: None,
                    to: None,
                };
                self.messages.push(msg);
                self.clear_input();
                return None;
            }
            // No match
            let msg = Message {
                from: "bpmux".to_string(),
                text: format!("no channel matches \"{arg}\". Type /view to list channels."),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text == "/pins" {
            let list = if self.pins.is_empty() {
                "No pins yet. Use /pin <text> to pin a decision.".to_string()
            } else {
                let items: Vec<String> = self
                    .pins
                    .iter()
                    .enumerate()
                    .map(|(i, p)| format!("  {}. {}", i + 1, p))
                    .collect();
                format!("Pinned:\n{}", items.join("\n"))
            };
            let msg = Message {
                from: "bpmux".to_string(),
                text: list,
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if let Some(arg) = text.strip_prefix("/pin ") {
            let arg = arg.trim();
            if arg.is_empty() {
                let msg = Message {
                    from: "bpmux".to_string(),
                    text: "Usage: /pin <text>. Example: /pin channels-as-views because filter-on-render".to_string(),
                    ts: Utc::now(),
                    message_type: None,
                    to: None,
                };
                self.messages.push(msg);
                self.clear_input();
                return None;
            }
            // Duplicate guard: case-insensitive exact match
            if let Some(pos) = self
                .pins
                .iter()
                .position(|p| p.eq_ignore_ascii_case(arg))
            {
                let msg = Message {
                    from: "bpmux".to_string(),
                    text: format!("already pinned as #{}", pos + 1),
                    ts: Utc::now(),
                    message_type: None,
                    to: None,
                };
                self.messages.push(msg);
                self.clear_input();
                return None;
            }
            if self.pins.len() >= 5 {
                let msg = Message {
                    from: "bpmux".to_string(),
                    text: "max 5 pins — unpin one first".to_string(),
                    ts: Utc::now(),
                    message_type: None,
                    to: None,
                };
                self.messages.push(msg);
                self.clear_input();
                return None;
            }
            self.pins.push(arg.to_string());
            if let Some(dir) = &self.session_dir {
                let _ = crate::session::log::write_pins(dir, &self.pins);
            }
            let pin_num = self.pins.len();
            let msg = Message {
                from: "bpmux".to_string(),
                text: format!("Pinned #{pin_num}: {arg}"),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            // Return a broadcast system message so all participants see pin changes
            return Some(Message {
                from: format!("{}:human", self.peer_name),
                text: format!("{} pinned #{pin_num}: {arg}", self.peer_name),
                ts: Utc::now(),
                message_type: Some("system".to_string()),
                to: None,
            });
        }
        if text == "/pin" {
            let msg = Message {
                from: "bpmux".to_string(),
                text:
                    "Usage: /pin <text>. Example: /pin channels-as-views because filter-on-render"
                        .to_string(),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if let Some(arg) = text.strip_prefix("/unpin ") {
            let arg = arg.trim();
            match arg.parse::<usize>() {
                Ok(n) if n >= 1 && n <= self.pins.len() => {
                    let removed = self.pins.remove(n - 1);
                    if let Some(dir) = &self.session_dir {
                        let _ = crate::session::log::write_pins(dir, &self.pins);
                    }
                    let msg = Message {
                        from: "bpmux".to_string(),
                        text: format!("Unpinned #{n}: {removed}"),
                        ts: Utc::now(),
                        message_type: None,
                        to: None,
                    };
                    self.messages.push(msg);
                    self.clear_input();
                    // Broadcast unpin to all participants
                    return Some(Message {
                        from: format!("{}:human", self.peer_name),
                        text: format!("{} unpinned #{n}: {removed}", self.peer_name),
                        ts: Utc::now(),
                        message_type: Some("system".to_string()),
                        to: None,
                    });
                }
                _ => {
                    let msg = Message {
                        from: "bpmux".to_string(),
                        text: format!("no pin #{arg}. Type /pins to list."),
                        ts: Utc::now(),
                        message_type: None,
                        to: None,
                    };
                    self.messages.push(msg);
                }
            }
            self.clear_input();
            return None;
        }
        if text == "/unpin" {
            let msg = Message {
                from: "bpmux".to_string(),
                text: "Usage: /unpin <number>. Type /pins to see pin numbers.".to_string(),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text == "/board" {
            let board_text = match Self::fetch_board() {
                Ok(text) => text,
                Err(e) => format!(
                    "Board unavailable: {e}\nEnsure `gh` CLI is installed and authenticated (`gh auth login`)."
                ),
            };
            let msg = Message {
                from: "bpmux".to_string(),
                text: board_text,
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text.starts_with('/') {
            let msg = Message {
                from: "bpmux".to_string(),
                text: format!(
                    "Unknown command: {}. Type /help for available commands.",
                    text
                ),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        // Derive `to` from send_target if set (sidebar selection)
        let to = self.send_target.as_ref().and_then(|ch| {
            if ch == "broadcast" {
                return None; // broadcast = no `to`
            }
            // Parse channel name back to recipient list, excluding self
            let names: Vec<String> = if ch.contains('\u{2194}') {
                ch.split('\u{2194}').map(String::from).collect()
            } else if ch.contains('+') {
                ch.split('+').map(String::from).collect()
            } else {
                return None;
            };
            let recipients: Vec<String> =
                names.into_iter().filter(|n| n != &self.peer_name).collect();
            if recipients.is_empty() {
                None
            } else {
                Some(recipients)
            }
        });
        let msg = Message {
            from: format!("{}:human", self.peer_name),
            text,
            ts: Utc::now(),
            message_type: None,
            to,
        };
        self.messages.push(msg.clone());
        self.clear_input();
        self.scroll_offset = 0;
        self.unread_count = 0;
        if self.input_history.len() >= MAX_HISTORY {
            self.input_history.remove(0);
        }
        self.input_history.push(msg.text.clone());
        self.history_index = None;
        Some(msg)
    }

    /// Tab-complete participant names after `/to `.
    /// Returns true if completion was applied, false if no match.
    pub fn tab_complete(&mut self) -> bool {
        let input = self.input.trim_start();
        let rest = match input.strip_prefix("/to ") {
            Some(r) => r,
            None => return false,
        };
        // Only complete in the recipients portion (before any space in the rest)
        if rest.contains(' ') {
            return false;
        }
        // Get the partial name being typed (after last comma, if CSV)
        let (prefix_part, partial) = match rest.rfind(',') {
            Some(i) => (&rest[..=i], rest[i + 1..].trim()),
            None => ("", rest.trim()),
        };
        // Collect known peer short names
        let mut candidates: Vec<String> = self
            .peer_activity
            .keys()
            .map(|k| k.split(':').next().unwrap_or(k).to_string())
            .filter(|name| name.starts_with(partial) && name != partial)
            .collect();
        candidates.sort();
        if candidates.is_empty() {
            return false;
        }
        // Pick first match (simple, no cycling for now)
        let completed = &candidates[0];
        self.input = format!("/to {}{} ", prefix_part, completed);
        self.cursor_pos = self.input.len();
        true
    }

    /// Fetch project board from GitHub Issues via `gh` CLI.
    fn fetch_board() -> Result<String, String> {
        let mut cmd = std::process::Command::new("gh");
        cmd.args([
            "issue",
            "list",
            "--state",
            "all",
            "--limit",
            "50",
            "--json",
            "number,title,state,labels",
        ]);
        // Use BPMUX_REPO env var if set, so /board works outside the git repo
        if let Ok(repo) = std::env::var("BPMUX_REPO") {
            cmd.args(["-R", &repo]);
        }
        let output = cmd.output().map_err(|e| format!("failed to run gh: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("gh failed: {stderr}"));
        }
        let json: Vec<serde_json::Value> =
            serde_json::from_slice(&output.stdout).map_err(|e| format!("parse error: {e}"))?;

        let mut active: Vec<String> = Vec::new();
        let mut backlog: Vec<String> = Vec::new();
        let mut icebox: Vec<String> = Vec::new();
        let mut done: Vec<String> = Vec::new();

        for issue in &json {
            let num = issue["number"].as_u64().unwrap_or(0);
            let title = issue["title"].as_str().unwrap_or("?");
            let state = issue["state"].as_str().unwrap_or("OPEN");
            let labels: Vec<&str> = issue["labels"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
                .unwrap_or_default();

            // Truncate title to fit 80 cols: "  #NN Title..." = ~74 chars for title
            let display_title: String = if title.len() > 65 {
                format!("{}...", &title[..62])
            } else {
                title.to_string()
            };
            let line = format!("  #{num} {display_title}");

            if state == "CLOSED" {
                done.push(line);
            } else if labels.contains(&"icebox") {
                icebox.push(line);
            } else if labels.contains(&"backlog") {
                backlog.push(line);
            } else {
                active.push(line);
            }
        }

        let mut sections: Vec<String> = Vec::new();
        if !active.is_empty() {
            sections.push(format!("Active ({})\n{}", active.len(), active.join("\n")));
        }
        if !backlog.is_empty() {
            sections.push(format!(
                "Backlog ({})\n{}",
                backlog.len(),
                backlog
                    .iter()
                    .take(10)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        if !icebox.is_empty() {
            sections.push(format!(
                "Icebox ({})\n{}",
                icebox.len(),
                icebox
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        if !done.is_empty() {
            sections.push(format!(
                "Done ({})\n{}",
                done.len(),
                done.iter().take(10).cloned().collect::<Vec<_>>().join("\n")
            ));
        }

        if sections.is_empty() {
            Ok("No issues found.".to_string())
        } else {
            Ok(format!("Board\n{}", sections.join("\n\n")))
        }
    }

    /// Returns the filtered list of slash commands matching current input.
    pub fn popup_matches(&self) -> Vec<(&'static str, &'static str)> {
        if !self.input.starts_with('/') || self.input.contains(' ') {
            return vec![];
        }
        let typed = self.input.split_whitespace().next().unwrap_or(&self.input);
        SLASH_COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(typed))
            .copied()
            .collect()
    }

    /// Returns true if the popup is currently visible.
    pub fn popup_visible(&self) -> bool {
        !self.popup_matches().is_empty()
    }

    /// Move popup selection up.
    pub fn popup_up(&mut self) {
        let count = self.popup_matches().len();
        if count == 0 {
            return;
        }
        self.popup_selected = Some(match self.popup_selected {
            None | Some(0) => count - 1,
            Some(i) => i - 1,
        });
    }

    /// Move popup selection down.
    pub fn popup_down(&mut self) {
        let count = self.popup_matches().len();
        if count == 0 {
            return;
        }
        self.popup_selected = Some(match self.popup_selected {
            None => 0,
            Some(i) if i + 1 >= count => 0,
            Some(i) => i + 1,
        });
    }

    /// Accept the currently selected popup item — fills input with the command + trailing space.
    /// Returns true if a selection was applied.
    pub fn popup_accept(&mut self) -> bool {
        let matches = self.popup_matches();
        if let Some(idx) = self.popup_selected
            && let Some((cmd, _)) = matches.get(idx)
        {
            self.input = format!("{} ", cmd);
            self.cursor_pos = self.input.len();
            self.popup_selected = None;
            return true;
        }
        false
    }

    /// Dismiss the popup without selecting.
    pub fn popup_dismiss(&mut self) {
        self.popup_selected = None;
    }

    /// Sidebar entries: project root node at index 0, channels indented below when expanded.
    /// Index 0 is always the project node (session_id). Selecting it shows all messages.
    pub fn sidebar_channels(&self) -> Vec<String> {
        let mut result = vec![self.session_id.clone()];
        if self.project_expanded {
            let mut channels: Vec<String> = self.active_channels.iter().cloned().collect();
            channels.sort();
            result.extend(channels);
        }
        result
    }

    /// Toggle sidebar open/closed. When opening, focus moves to sidebar.
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_open = !self.sidebar_open;
        self.sidebar_focused = self.sidebar_open;
    }

    /// Move sidebar selection up (wraps around).
    pub fn sidebar_up(&mut self) {
        let count = self.sidebar_channels().len();
        if count == 0 {
            return;
        }
        self.sidebar_selected = if self.sidebar_selected == 0 {
            count - 1
        } else {
            self.sidebar_selected - 1
        };
    }

    /// Move sidebar selection down (wraps around).
    pub fn sidebar_down(&mut self) {
        let count = self.sidebar_channels().len();
        if count == 0 {
            return;
        }
        self.sidebar_selected = (self.sidebar_selected + 1) % count;
    }

    /// Select the current sidebar item: sets view filter and send target.
    /// Index 0 (project node) = show all. Index 1+ = specific channel.
    pub fn sidebar_select(&mut self) {
        let channels = self.sidebar_channels();
        if let Some(ch) = channels.get(self.sidebar_selected) {
            if self.sidebar_selected == 0 {
                // Project root = all messages (deselected state)
                self.current_view = None;
                self.send_target = None;
            } else {
                self.current_view = Some(ch.clone());
                self.send_target = Some(ch.clone());
                self.channel_unread.remove(ch);
            }
            self.scroll_offset = 0;
        }
        // Return focus to input after selection
        self.sidebar_focused = false;
    }

    /// Collapse the project tree (Left arrow on project node).
    pub fn sidebar_collapse(&mut self) {
        if self.project_expanded {
            self.project_expanded = false;
            self.sidebar_selected = 0; // snap to project node
        }
    }

    /// Expand the project tree (Right arrow on project node).
    pub fn sidebar_expand(&mut self) {
        self.project_expanded = true;
    }

    /// Close sidebar and return focus to input.
    pub fn sidebar_close(&mut self) {
        self.sidebar_open = false;
        self.sidebar_focused = false;
    }

    pub fn history_up(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let new_index = match self.history_index {
            None => self.input_history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.history_index = Some(new_index);
        self.input = self.input_history[new_index].clone();
    }

    pub fn history_down(&mut self) {
        let Some(current) = self.history_index else {
            return;
        };
        if current + 1 >= self.input_history.len() {
            self.history_index = None;
            self.clear_input();
        } else {
            let new_index = current + 1;
            self.history_index = Some(new_index);
            self.input = self.input_history[new_index].clone();
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn submits_message_and_clears_input() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "hello world".to_string();
        let msg = app.submit_message().unwrap();
        assert_eq!(msg.from, "ryan:human");
        assert_eq!(msg.text, "hello world");
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn ignores_empty_input() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "   ".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
    }

    #[test]
    fn quit_command_sets_flag() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/quit".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.should_quit);
    }

    #[test]
    fn help_command_adds_message_and_clears_input() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/help".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].from, "bpmux");
        assert!(app.messages[0].text.contains("/help"));
        assert!(app.messages[0].text.contains("/quit"));
        assert!(app.messages[0].text.contains("test-session"));
    }

    #[test]
    fn scroll_up_increases_offset() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.max_scroll = 100;
        app.scroll_up(5);
        assert_eq!(app.scroll_offset, 5);
        app.scroll_up(3);
        assert_eq!(app.scroll_offset, 8);
    }

    #[test]
    fn scroll_up_clamps_to_max_scroll() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.max_scroll = 10;
        app.scroll_up(15);
        assert_eq!(app.scroll_offset, 10);
    }

    #[test]
    fn scroll_down_decreases_offset_to_zero() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.scroll_offset = 10;
        app.scroll_down(3);
        assert_eq!(app.scroll_offset, 7);
        app.scroll_down(10);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn receiving_message_when_scrolled_up_increments_unread_count() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.scroll_offset = 5;
        app.receive_message(Message {
            from: "bot:ws".to_string(),
            text: "hello".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        });
        assert_eq!(app.unread_count, 1);
        app.receive_message(Message {
            from: "bot:ws".to_string(),
            text: "world".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        });
        assert_eq!(app.unread_count, 2);
    }

    #[test]
    fn scrolling_to_bottom_resets_unread_count() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.scroll_offset = 3;
        app.unread_count = 5;
        app.scroll_down(3);
        assert_eq!(app.scroll_offset, 0);
        assert_eq!(app.unread_count, 0);
    }

    #[test]
    fn receiving_message_at_bottom_does_not_increment_unread_count() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        assert_eq!(app.scroll_offset, 0);
        app.receive_message(Message {
            from: "bot:ws".to_string(),
            text: "hello".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        });
        assert_eq!(app.unread_count, 0);
    }

    #[test]
    fn incoming_message_stays_at_bottom_when_at_bottom() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        assert_eq!(app.scroll_offset, 0);
        app.receive_message(Message {
            from: "bot:ws".to_string(),
            text: "hello".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        });
        assert_eq!(app.scroll_offset, 0);
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn up_arrow_recalls_last_sent_message() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "hello".to_string();
        app.submit_message();
        app.input = "world".to_string();
        app.submit_message();

        app.history_up();
        assert_eq!(app.input, "world");
        assert_eq!(app.history_index, Some(1));

        app.history_up();
        assert_eq!(app.input, "hello");
        assert_eq!(app.history_index, Some(0));
    }

    #[test]
    fn down_arrow_returns_to_empty_after_recalling() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "hello".to_string();
        app.submit_message();

        app.history_up();
        assert_eq!(app.input, "hello");

        app.history_down();
        assert!(app.input.is_empty());
        assert_eq!(app.history_index, None);
    }

    #[test]
    fn history_caps_at_50_entries() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        for i in 0..60 {
            app.input = format!("msg {i}");
            app.submit_message();
        }
        assert_eq!(app.input_history.len(), MAX_HISTORY);
        assert_eq!(app.input_history[0], "msg 10");
        assert_eq!(app.input_history[49], "msg 59");
    }

    #[test]
    fn who_command_lists_peer_name_and_returns_none() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/who".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].from, "bpmux");
        assert!(app.messages[0].text.contains("ryan (you)"));
    }

    #[test]
    fn who_command_includes_peer_activity_and_connected_count() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        app.connected_peers = 2;
        app.input = "/who".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.messages[0].text.contains("dev [working]"));
        assert!(app.messages[0].text.contains("2 connected"));
    }

    #[test]
    fn unrecognized_command_adds_error_message_without_broadcast() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/foo".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].from, "bpmux");
        assert!(app.messages[0].text.contains("Unknown command"));
        assert!(app.messages[0].text.contains("/foo"));
        assert!(app.messages[0].text.contains("/help"));
    }

    #[test]
    fn incoming_message_preserves_viewport_when_scrolled_up() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.scroll_offset = 5;
        app.receive_message(Message {
            from: "bot:ws".to_string(),
            text: "hello".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        });
        assert_eq!(app.scroll_offset, 6);
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn update_activity_adds_entry() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        assert!(app.peer_activity.contains_key("dev:claude-code"));
        let (status, _) = &app.peer_activity["dev:claude-code"];
        assert_eq!(status, "working");
    }

    #[test]
    fn activity_message_does_not_appear_in_messages() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.receive_message(Message {
            from: "dev:claude-code".to_string(),
            text: "working".to_string(),
            ts: Utc::now(),
            message_type: Some("activity".to_string()),
            to: None,
        });
        assert!(app.messages.is_empty());
        assert!(app.peer_activity.contains_key("dev:claude-code"));
    }

    #[test]
    fn debug_toggles_show_system_messages() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        assert!(!app.show_system_messages);

        app.input = "/debug".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.show_system_messages);
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages[0].text.contains("Debug mode on"));
        assert!(app.messages[0].text.contains("visible"));

        app.input = "/debug".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(!app.show_system_messages);
        assert_eq!(app.messages.len(), 2);
        assert!(app.messages[1].text.contains("Debug mode off"));
        assert!(app.messages[1].text.contains("hidden"));
    }

    #[test]
    fn help_includes_debug_command() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/help".to_string();
        app.submit_message();
        assert!(app.messages[0].text.contains("/debug"));
    }

    #[test]
    fn help_includes_to_command() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/help".to_string();
        app.submit_message();
        assert!(app.messages[0].text.contains("/to"));
    }

    #[test]
    fn to_single_recipient_returns_directed_message() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/to dev hello there".to_string();
        let msg = app.submit_message().unwrap();
        assert_eq!(msg.from, "ryan:human");
        assert_eq!(msg.text, "hello there");
        assert_eq!(msg.to, Some(vec!["dev".to_string()]));
    }

    #[test]
    fn to_csv_recipients_returns_directed_message() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/to dev,designer check this".to_string();
        let msg = app.submit_message().unwrap();
        assert_eq!(msg.text, "check this");
        assert_eq!(
            msg.to,
            Some(vec!["dev".to_string(), "designer".to_string()])
        );
    }

    #[test]
    fn to_bare_shows_usage() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/to".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.messages[0].text.contains("Usage:"));
    }

    #[test]
    fn to_recipient_no_body_shows_usage() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/to dev".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.messages[0].text.contains("Usage:"));
    }

    #[test]
    fn tab_completes_participant_name_after_to() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        app.update_activity("designer:claude-code".to_string(), "idle".to_string());
        app.input = "/to de".to_string();
        app.cursor_pos = app.input.len();
        assert!(app.tab_complete());
        // Should complete to first alphabetical match
        assert!(
            app.input.starts_with("/to de"),
            "should complete a 'de' prefix, got: {}",
            app.input
        );
        assert!(
            app.input.ends_with(' '),
            "should add trailing space for body"
        );
    }

    #[test]
    fn tab_completes_csv_second_recipient() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        app.update_activity("pm:claude-code".to_string(), "idle".to_string());
        app.input = "/to dev,p".to_string();
        app.cursor_pos = app.input.len();
        assert!(app.tab_complete());
        assert_eq!(app.input, "/to dev,pm ");
    }

    #[test]
    fn tab_does_nothing_without_to_prefix() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "hello".to_string();
        app.cursor_pos = app.input.len();
        assert!(!app.tab_complete());
        assert_eq!(app.input, "hello");
    }

    #[test]
    fn tab_does_nothing_in_body_portion() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        app.input = "/to dev hel".to_string();
        app.cursor_pos = app.input.len();
        assert!(!app.tab_complete());
    }

    #[test]
    fn dm_increments_unread_and_sets_flash() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.receive_message(Message {
            from: "pm:claude-code".to_string(),
            text: "hey ryan".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["ryan".to_string()]),
        });
        assert_eq!(app.dm_unread_from.len(), 1);
        assert!(app.dm_flash_until.is_some());
    }

    #[test]
    fn broadcast_does_not_increment_dm_unread() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.receive_message(Message {
            from: "pm:claude-code".to_string(),
            text: "hello all".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        });
        assert_eq!(app.dm_unread_from.len(), 0);
        assert!(app.dm_flash_until.is_none());
    }

    #[test]
    fn to_all_does_not_increment_dm_unread() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.receive_message(Message {
            from: "pm:claude-code".to_string(),
            text: "hello".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["all".to_string()]),
        });
        assert_eq!(app.dm_unread_from.len(), 0);
    }

    #[test]
    fn dm_to_other_does_not_increment_dm_unread() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.receive_message(Message {
            from: "pm:claude-code".to_string(),
            text: "hey dev".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["dev".to_string()]),
        });
        assert_eq!(app.dm_unread_from.len(), 0);
    }

    #[test]
    fn submit_clears_dm_unread() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.receive_message(Message {
            from: "pm:claude-code".to_string(),
            text: "hey".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["ryan".to_string()]),
        });
        assert_eq!(app.dm_unread_from.len(), 1);
        app.input = "got it".to_string();
        app.submit_message();
        assert_eq!(app.dm_unread_from.len(), 0);
        assert!(app.dm_flash_until.is_none());
    }

    #[test]
    fn broadcast_message_adds_channel() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        assert_eq!(app.active_channels.len(), 0);
        app.receive_message(Message {
            from: "dev:claude-code".to_string(),
            text: "hello".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        });
        assert_eq!(app.active_channels.len(), 1);
        assert!(app.active_channels.contains("broadcast"));
    }

    #[test]
    fn directed_message_adds_new_channel() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.receive_message(Message {
            from: "dev:claude-code".to_string(),
            text: "broadcast".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        });
        app.receive_message(Message {
            from: "dev:claude-code".to_string(),
            text: "hey".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["ryan".to_string()]),
        });
        assert_eq!(app.active_channels.len(), 2);
    }

    #[test]
    fn same_channel_does_not_duplicate() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.receive_message(Message {
            from: "dev:claude-code".to_string(),
            text: "hey".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["ryan".to_string()]),
        });
        app.receive_message(Message {
            from: "dev:claude-code".to_string(),
            text: "also hey".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["ryan".to_string()]),
        });
        assert_eq!(app.active_channels.len(), 1);
        assert!(app.active_channels.contains("dev\u{2194}ryan"));
    }

    #[test]
    fn different_senders_create_different_channels() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.receive_message(Message {
            from: "dev:claude-code".to_string(),
            text: "hey".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["ryan".to_string()]),
        });
        app.receive_message(Message {
            from: "pm:claude-code".to_string(),
            text: "also hey".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["ryan".to_string()]),
        });
        assert_eq!(app.active_channels.len(), 2);
        assert!(app.active_channels.contains("dev\u{2194}ryan"));
        assert!(app.active_channels.contains("pm\u{2194}ryan"));
    }

    #[test]
    fn channel_name_broadcast() {
        let msg = Message {
            from: "dev:claude-code".to_string(),
            text: "hi".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        };
        assert_eq!(channel_name(&msg), "broadcast");
    }

    #[test]
    fn channel_name_to_all_is_broadcast() {
        let msg = Message {
            from: "dev:claude-code".to_string(),
            text: "hi".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["all".to_string()]),
        };
        assert_eq!(channel_name(&msg), "broadcast");
    }

    #[test]
    fn channel_name_dm() {
        let msg = Message {
            from: "dev:claude-code".to_string(),
            text: "hey".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["ryan".to_string()]),
        };
        assert_eq!(channel_name(&msg), "dev\u{2194}ryan");
    }

    #[test]
    fn channel_name_group() {
        let msg = Message {
            from: "pm:claude-code".to_string(),
            text: "hey".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["dev".to_string(), "ryan".to_string()]),
        };
        assert_eq!(channel_name(&msg), "dev+pm+ryan");
    }

    #[test]
    fn view_broadcast_sets_current_view() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/view broadcast".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert_eq!(app.current_view, Some("broadcast".to_string()));
    }

    #[test]
    fn view_all_clears_current_view() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.current_view = Some("broadcast".to_string());
        app.input = "/view all".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert_eq!(app.current_view, None);
    }

    #[test]
    fn view_known_channel_sets_current_view() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.active_channels.insert("dev\u{2194}ryan".to_string());
        app.input = "/view dev\u{2194}ryan".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert_eq!(app.current_view, Some("dev\u{2194}ryan".to_string()));
    }

    #[test]
    fn view_unknown_channel_shows_error() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        let original_view = app.current_view.clone();
        app.input = "/view fake".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert_eq!(app.current_view, original_view);
        assert!(
            app.messages
                .last()
                .unwrap()
                .text
                .contains("no channel matches")
        );
    }

    #[test]
    fn bare_view_lists_channels() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.active_channels.insert("broadcast".to_string());
        app.active_channels.insert("dev\u{2194}ryan".to_string());
        app.input = "/view".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(
            app.messages
                .last()
                .unwrap()
                .text
                .contains("Available channels")
        );
        assert!(app.messages.last().unwrap().text.contains("broadcast"));
    }

    #[test]
    fn activity_messages_do_not_create_channels() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.receive_message(Message {
            from: "dev:claude-code".to_string(),
            text: "busy".to_string(),
            ts: Utc::now(),
            message_type: Some("activity".to_string()),
            to: None,
        });
        assert_eq!(app.active_channels.len(), 0);
    }

    #[test]
    fn popup_matches_filters_by_typed_prefix() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/v".to_string();
        let m = app.popup_matches();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].0, "/view");
    }

    #[test]
    fn popup_matches_returns_all_on_bare_slash() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/".to_string();
        assert_eq!(app.popup_matches().len(), SLASH_COMMANDS.len());
    }

    #[test]
    fn popup_matches_empty_after_space() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/to dev".to_string();
        assert!(app.popup_matches().is_empty());
    }

    #[test]
    fn popup_navigation_wraps_around() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/".to_string();
        let count = app.popup_matches().len();

        // Down from None starts at 0
        app.popup_down();
        assert_eq!(app.popup_selected, Some(0));

        // Up from 0 wraps to last
        app.popup_up();
        assert_eq!(app.popup_selected, Some(count - 1));

        // Down from last wraps to 0
        app.popup_down();
        assert_eq!(app.popup_selected, Some(0));
    }

    #[test]
    fn popup_accept_fills_input() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/t".to_string();
        app.popup_down(); // select /to
        assert!(app.popup_accept());
        assert_eq!(app.input, "/to ");
        assert_eq!(app.cursor_pos, 4);
        assert!(app.popup_selected.is_none());
    }

    #[test]
    fn popup_selected_resets_on_input_change() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/".to_string();
        app.cursor_pos = 1;
        app.popup_down();
        assert!(app.popup_selected.is_some());
        app.insert_char('t');
        assert!(app.popup_selected.is_none());
    }

    #[test]
    fn pin_adds_to_list() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/pin channels-as-views because filter-on-render".to_string();
        app.submit_message();
        assert_eq!(app.pins.len(), 1);
        assert_eq!(app.pins[0], "channels-as-views because filter-on-render");
        assert!(app.messages.last().unwrap().text.contains("Pinned #1"));
    }

    #[test]
    fn pin_max_five() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        for i in 1..=5 {
            app.input = format!("/pin decision {i}");
            app.submit_message();
        }
        assert_eq!(app.pins.len(), 5);
        app.input = "/pin one too many".to_string();
        app.submit_message();
        assert_eq!(app.pins.len(), 5);
        assert!(app.messages.last().unwrap().text.contains("max 5 pins"));
    }

    #[test]
    fn unpin_removes_by_number() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/pin first".to_string();
        app.submit_message();
        app.input = "/pin second".to_string();
        app.submit_message();
        app.input = "/unpin 1".to_string();
        app.submit_message();
        assert_eq!(app.pins.len(), 1);
        assert_eq!(app.pins[0], "second");
        assert!(app.messages.last().unwrap().text.contains("Unpinned #1"));
    }

    #[test]
    fn unpin_invalid_number_shows_error() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/pin something".to_string();
        app.submit_message();
        app.input = "/unpin 5".to_string();
        app.submit_message();
        assert_eq!(app.pins.len(), 1);
        assert!(app.messages.last().unwrap().text.contains("no pin #5"));
    }

    #[test]
    fn pins_command_lists_pins() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/pin theory A".to_string();
        app.submit_message();
        app.input = "/pin theory B".to_string();
        app.submit_message();
        app.input = "/pins".to_string();
        app.submit_message();
        let last = &app.messages.last().unwrap().text;
        assert!(last.contains("1. theory A"));
        assert!(last.contains("2. theory B"));
    }

    #[test]
    fn pins_empty_shows_hint() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/pins".to_string();
        app.submit_message();
        assert!(app.messages.last().unwrap().text.contains("No pins yet"));
    }

    #[test]
    fn pin_bare_shows_usage() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/pin".to_string();
        app.submit_message();
        assert!(app.messages.last().unwrap().text.contains("Usage:"));
    }

    #[test]
    fn unpin_bare_shows_usage() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/unpin".to_string();
        app.submit_message();
        assert!(app.messages.last().unwrap().text.contains("Usage:"));
    }

    #[test]
    fn pin_duplicate_rejected() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/pin channels-as-views".to_string();
        app.submit_message();
        app.input = "/pin Channels-As-Views".to_string();
        app.submit_message();
        assert_eq!(app.pins.len(), 1);
        assert!(app
            .messages
            .last()
            .unwrap()
            .text
            .contains("already pinned as #1"));
    }

    #[test]
    fn pin_broadcasts_system_message() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/pin test pin".to_string();
        let msg = app.submit_message();
        assert!(msg.is_some(), "pin should return a broadcast message");
        let msg = msg.unwrap();
        assert!(msg.text.contains("ryan pinned #1: test pin"));
        assert_eq!(msg.message_type.as_deref(), Some("system"));
    }

    #[test]
    fn unpin_broadcasts_system_message() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input = "/pin to remove".to_string();
        app.submit_message();
        app.input = "/unpin 1".to_string();
        let msg = app.submit_message();
        assert!(msg.is_some(), "unpin should return a broadcast message");
        let msg = msg.unwrap();
        assert!(msg.text.contains("ryan unpinned #1: to remove"));
        assert_eq!(msg.message_type.as_deref(), Some("system"));
    }

    // Sidebar tests

    #[test]
    fn sidebar_channels_starts_with_project_node() {
        let app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        let channels = app.sidebar_channels();
        assert_eq!(channels[0], "test-session");
    }

    #[test]
    fn sidebar_channels_includes_active_channels() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.active_channels.insert("broadcast".to_string());
        app.active_channels.insert("dev↔human".to_string());
        let channels = app.sidebar_channels();
        assert!(channels.contains(&"broadcast".to_string()));
        assert!(channels.contains(&"dev↔human".to_string()));
    }

    #[test]
    fn sidebar_toggle_opens_and_focuses() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        assert!(!app.sidebar_open);
        app.toggle_sidebar();
        assert!(app.sidebar_open);
        assert!(app.sidebar_focused);
    }

    #[test]
    fn sidebar_select_all_clears_view_and_target() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.current_view = Some("broadcast".to_string());
        app.send_target = Some("broadcast".to_string());
        app.sidebar_selected = 0; // project node is always first
        app.sidebar_select();
        assert!(app.current_view.is_none());
        assert!(app.send_target.is_none());
    }

    #[test]
    fn sidebar_select_channel_sets_view_and_target() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.active_channels.insert("broadcast".to_string());
        app.sidebar_selected = 1; // "broadcast" (sorted after project node)
        app.sidebar_select();
        assert_eq!(app.current_view, Some("broadcast".to_string()));
        assert_eq!(app.send_target, Some("broadcast".to_string()));
    }

    #[test]
    fn sidebar_select_clears_unread() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.active_channels.insert("broadcast".to_string());
        app.channel_unread.insert("broadcast".to_string(), 5);
        app.sidebar_selected = 1;
        app.sidebar_select();
        assert_eq!(app.channel_unread.get("broadcast"), None);
    }

    #[test]
    fn sidebar_navigation_wraps() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.active_channels.insert("broadcast".to_string());
        // channels: ["test", "broadcast"]
        app.sidebar_selected = 0;
        app.sidebar_up(); // wraps to last
        assert_eq!(app.sidebar_selected, 1);
        app.sidebar_down(); // wraps to first
        assert_eq!(app.sidebar_selected, 0);
    }

    #[test]
    fn sidebar_close_unfocuses() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.toggle_sidebar();
        assert!(app.sidebar_open);
        app.sidebar_close();
        assert!(!app.sidebar_open);
        assert!(!app.sidebar_focused);
    }

    #[test]
    fn sidebar_collapse_hides_channels() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.active_channels.insert("broadcast".to_string());
        assert!(app.project_expanded);
        assert_eq!(app.sidebar_channels().len(), 2); // project + broadcast
        app.sidebar_collapse();
        assert!(!app.project_expanded);
        assert_eq!(app.sidebar_channels().len(), 1); // project only
        assert_eq!(app.sidebar_selected, 0); // snapped to project node
    }

    #[test]
    fn sidebar_expand_shows_channels() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.active_channels.insert("broadcast".to_string());
        app.project_expanded = false;
        assert_eq!(app.sidebar_channels().len(), 1);
        app.sidebar_expand();
        assert!(app.project_expanded);
        assert_eq!(app.sidebar_channels().len(), 2);
    }

    #[test]
    fn sidebar_expanded_by_default() {
        let app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        assert!(app.project_expanded);
    }

    #[test]
    fn send_target_directs_outgoing_messages() {
        let mut app = App::new("human".to_string(), "test".to_string(), vec![]);
        app.send_target = Some("dev↔human".to_string());
        app.input = "hello dev".to_string();
        app.cursor_pos = app.input.len();
        let msg = app.submit_message().unwrap();
        assert_eq!(msg.to, Some(vec!["dev".to_string()]));
    }

    #[test]
    fn no_send_target_broadcasts() {
        let mut app = App::new("human".to_string(), "test".to_string(), vec![]);
        app.input = "hello everyone".to_string();
        app.cursor_pos = app.input.len();
        let msg = app.submit_message().unwrap();
        assert!(msg.to.is_none());
    }

    #[test]
    fn channel_unread_increments_on_receive() {
        let mut app = App::new("human".to_string(), "test".to_string(), vec![]);
        app.current_view = Some("broadcast".to_string());
        let msg = Message {
            from: "dev:claude-code".to_string(),
            text: "dm message".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(vec!["human".to_string()]),
        };
        app.receive_message(msg);
        let ch = "dev↔human";
        assert_eq!(*app.channel_unread.get(ch).unwrap_or(&0), 1);
    }
}
