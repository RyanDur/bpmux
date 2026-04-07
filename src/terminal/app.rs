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
    ("/who", "list participants"),
    ("/debug", "toggle system messages"),
    ("/quit", "exit bpmux"),
];

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
}

impl App {
    pub fn new(peer_name: String, session_id: String, messages: Vec<Message>) -> Self {
        let mut active_channels: HashSet<String> = HashSet::new();
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

    fn system_message(&mut self, text: String) {
        self.messages.push(Message {
            from: "bpmux".to_string(),
            text,
            ts: Utc::now(),
            message_type: None,
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
            let ch = channel_name(&msg, &self.peer_name);
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

        if text == "/who" {
            return self.handle_who();
        }
        if text == "/quit" {
            return self.handle_quit();
        }
        if text == "/debug" {
            return self.handle_debug();
        }
        if text.starts_with("/status ") {
            return self.handle_status(text);
        }
        if let Some(rest) = text.strip_prefix("/to ") {
            return self.handle_to_with_args(rest.to_string());
        }
        if text == "/to" {
            return self.handle_to_bare();
        }
        if text == "/view" {
            return self.handle_view_bare();
        }
        if let Some(arg) = text.strip_prefix("/view ") {
            return self.handle_view_with_arg(arg.to_string());
        }
        if text == "/pins" {
            return self.handle_pins();
        }
        if let Some(arg) = text.strip_prefix("/pin ") {
            return self.handle_pin_with_arg(arg.to_string());
        }
        if text == "/pin" {
            return self.handle_pin_bare();
        }
        if let Some(arg) = text.strip_prefix("/unpin ") {
            return self.handle_unpin_with_arg(arg.to_string());
        }
        if text == "/unpin" {
            return self.handle_unpin_bare();
        }
        if text == "/board" {
            return self.handle_board();
        }
        if text.starts_with('/') {
            return self.handle_unknown_command(text);
        }

        self.send_chat_message(text)
    }

    fn handle_who(&mut self) -> Option<Message> {
        let mut participants = vec![format!("{} (you)", self.peer_name)];
        for (peer, (status, _)) in &self.peer_activity {
            let short = peer.split(':').next().unwrap_or(peer);
            participants.push(format!("{} [{}]", short, status));
        }
        if self.connected_peers > 0 {
            participants.push(format!("{} connected", self.connected_peers));
        }
        self.system_message(format!("Participants:\n  {}", participants.join("\n  ")));
        self.clear_input();
        None
    }

    fn handle_quit(&mut self) -> Option<Message> {
        self.should_quit = true;
        None
    }

    fn handle_debug(&mut self) -> Option<Message> {
        self.show_system_messages = !self.show_system_messages;
        let state = if self.show_system_messages {
            "on"
        } else {
            "off"
        };
        self.system_message(format!(
            "Debug mode {state}. System messages are now {}.",
            if self.show_system_messages {
                "visible"
            } else {
                "hidden"
            }
        ));
        self.clear_input();
        None
    }

    fn handle_status(&mut self, text: String) -> Option<Message> {
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
        Some(msg)
    }

    fn handle_to_with_args(&mut self, rest: String) -> Option<Message> {
        let rest = rest.trim_start();
        if let Some((recipients_str, body)) = rest.split_once(' ') {
            let body = body.trim().to_string();
            if body.is_empty() {
                self.system_message(
                    "Usage: /to <recipients> <message>. Example: /to dev,pm hello".to_string(),
                );
                self.clear_input();
                return None;
            }
            let recipients: Vec<String> = recipients_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if recipients.is_empty() {
                self.system_message(
                    "Usage: /to <recipients> <message>. Example: /to dev,pm hello".to_string(),
                );
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
        self.system_message(
            "Usage: /to <recipients> <message>. Example: /to dev,pm hello".to_string(),
        );
        self.clear_input();
        None
    }

    fn handle_to_bare(&mut self) -> Option<Message> {
        self.system_message(
            "Usage: /to <recipients> <message>. Example: /to dev,pm hello".to_string(),
        );
        self.clear_input();
        None
    }

    fn handle_view_bare(&mut self) -> Option<Message> {
        // List available channels
        let mut channel_names: Vec<&str> =
            self.active_channels.iter().map(|s| s.as_str()).collect();
        channel_names.sort();
        let list = if channel_names.is_empty() {
            "No channels yet.".to_string()
        } else {
            format!("Available channels: {}", channel_names.join(", "))
        };
        self.system_message(list);
        self.clear_input();
        None
    }

    fn handle_view_with_arg(&mut self, arg: String) -> Option<Message> {
        let arg = arg.trim();
        if arg == "all" {
            self.current_view = None;
            self.scroll_offset = 0;
            self.system_message("View set to all channels.".to_string());
            self.clear_input();
            return None;
        }
        if arg == "broadcast" {
            self.current_view = Some("broadcast".to_string());
            self.scroll_offset = 0;
            self.system_message("View set to broadcast.".to_string());
            self.clear_input();
            return None;
        }
        // Check if arg matches a known channel
        if self.active_channels.contains(arg) {
            self.current_view = Some(arg.to_string());
            self.scroll_offset = 0;
            self.system_message(format!("View set to {arg}."));
            self.clear_input();
            return None;
        }
        // No match
        self.system_message(format!(
            "no channel matches \"{arg}\". Type /view to list channels."
        ));
        self.clear_input();
        None
    }

    fn handle_pins(&mut self) -> Option<Message> {
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
        self.system_message(list);
        self.clear_input();
        None
    }

    fn handle_pin_with_arg(&mut self, arg: String) -> Option<Message> {
        let arg = arg.trim();
        if arg.is_empty() {
            self.system_message(
                "Usage: /pin <text>. Example: /pin channels-as-views because filter-on-render"
                    .to_string(),
            );
            self.clear_input();
            return None;
        }
        if self.pins.len() >= 5 {
            self.system_message("max 5 pins — /unpin one first".to_string());
            self.clear_input();
            return None;
        }
        self.pins.push(arg.to_string());
        if let Some(dir) = &self.session_dir {
            let _ = crate::session::log::write_pins(dir, &self.pins);
        }
        self.system_message(format!("Pinned #{}: {arg}", self.pins.len()));
        self.clear_input();
        None
    }

    fn handle_pin_bare(&mut self) -> Option<Message> {
        self.system_message(
            "Usage: /pin <text>. Example: /pin channels-as-views because filter-on-render"
                .to_string(),
        );
        self.clear_input();
        None
    }

    fn handle_unpin_with_arg(&mut self, arg: String) -> Option<Message> {
        let arg = arg.trim();
        match arg.parse::<usize>() {
            Ok(n) if n >= 1 && n <= self.pins.len() => {
                let removed = self.pins.remove(n - 1);
                if let Some(dir) = &self.session_dir {
                    let _ = crate::session::log::write_pins(dir, &self.pins);
                }
                self.system_message(format!("Unpinned #{n}: {removed}"));
            }
            _ => {
                self.system_message(format!("no pin #{arg}. Type /pins to list."));
            }
        }
        self.clear_input();
        None
    }

    fn handle_unpin_bare(&mut self) -> Option<Message> {
        self.system_message("Usage: /unpin <number>. Type /pins to see pin numbers.".to_string());
        self.clear_input();
        None
    }

    fn handle_board(&mut self) -> Option<Message> {
        let board_text = match Self::fetch_board() {
            Ok(text) => text,
            Err(e) => format!(
                "Board unavailable: {e}\nEnsure `gh` CLI is installed and authenticated (`gh auth login`)."
            ),
        };
        self.system_message(board_text);
        self.clear_input();
        None
    }

    fn handle_unknown_command(&mut self, text: String) -> Option<Message> {
        self.system_message(format!("Unknown command: {}. Press ? for help.", text));
        self.clear_input();
        None
    }

    fn send_chat_message(&mut self, text: String) -> Option<Message> {
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

        struct BoardItem {
            number: u64,
            title: String,
            status: Option<String>,
        }

        let mut active: Vec<BoardItem> = Vec::new();
        let mut backlog: Vec<BoardItem> = Vec::new();
        let mut icebox: Vec<BoardItem> = Vec::new();

        for issue in &json {
            let num = issue["number"].as_u64().unwrap_or(0);
            let title = issue["title"].as_str().unwrap_or("?").to_string();
            let state = issue["state"].as_str().unwrap_or("OPEN");
            let labels: Vec<&str> = issue["labels"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
                .unwrap_or_default();

            if state == "CLOSED" {
                continue; // Done items not shown
            }

            // Derive status from labels for active items
            let status = if labels.contains(&"delivered") {
                Some("delivered".to_string())
            } else if labels.contains(&"finished") {
                Some("finished".to_string())
            } else if labels.contains(&"started") {
                Some("started".to_string())
            } else if labels.contains(&"rejected") {
                Some("rejected".to_string())
            } else {
                None
            };

            let item = BoardItem {
                number: num,
                title,
                status,
            };

            if labels.contains(&"icebox") {
                icebox.push(item);
            } else if labels.contains(&"backlog") {
                backlog.push(item);
            } else {
                active.push(item);
            }
        }

        let mut lines: Vec<String> = Vec::new();
        lines.push("## Board".to_string());

        if !active.is_empty() {
            lines.push(String::new());
            lines.push(format!("### ● Active ({})", active.len()));
            for item in &active {
                let status_tag = item
                    .status
                    .as_ref()
                    .map(|s| format!(" *{s}*"))
                    .unwrap_or_default();
                lines.push(format!("- **#{}** {}{status_tag}", item.number, item.title));
            }
        }

        if !backlog.is_empty() {
            lines.push(String::new());
            lines.push(format!("### ○ Backlog ({})", backlog.len()));
            for item in backlog.iter().take(10) {
                lines.push(format!("- **#{}** {}", item.number, item.title));
            }
            if backlog.len() > 10 {
                lines.push(format!("- *+{} more*", backlog.len() - 10));
            }
        }

        if !icebox.is_empty() {
            lines.push(String::new());
            lines.push(format!("### ◌ Icebox ({})", icebox.len()));
            if let Some(first) = icebox.first() {
                lines.push(format!("- **#{}** {}", first.number, first.title));
            }
            if icebox.len() > 1 {
                lines.push(format!("- *+{} more*", icebox.len() - 1));
            }
        }

        if lines.len() <= 1 {
            Ok("No open issues found.".to_string())
        } else {
            Ok(lines.join("\n"))
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
        if !self.sidebar_open {
            // Closed → open + focused
            self.sidebar_open = true;
            self.sidebar_focused = true;
        } else if !self.sidebar_focused {
            // Open + unfocused → refocus
            self.sidebar_focused = true;
        } else {
            // Open + focused → close
            self.sidebar_open = false;
            self.sidebar_focused = false;
        }
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
            // Return focus to input so the user can type immediately
            self.sidebar_focused = false;
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

    /// Unfocus sidebar (return focus to input). Sidebar stays open.
    pub fn sidebar_close(&mut self) {
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

    fn test_app() -> App {
        App::new("ryan".to_string(), "test-session".to_string(), vec![])
    }

    fn chat_msg(from: &str, text: &str) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        }
    }

    fn directed_msg(from: &str, text: &str, to: Vec<&str>) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            message_type: None,
            to: Some(to.into_iter().map(String::from).collect()),
        }
    }

    fn system_msg(from: &str, text: &str) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            message_type: Some("system".to_string()),
            to: None,
        }
    }

    fn activity_msg(from: &str, text: &str) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            message_type: Some("activity".to_string()),
            to: None,
        }
    }

    #[test]
    fn submits_message_and_clears_input() {
        let mut app = test_app();
        app.input = "hello world".to_string();
        let msg = app.submit_message().unwrap();
        assert_eq!(msg.from, "ryan:human");
        assert_eq!(msg.text, "hello world");
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn ignores_empty_input() {
        let mut app = test_app();
        app.input = "   ".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
    }

    #[test]
    fn quit_command_sets_flag() {
        let mut app = test_app();
        app.input = "/quit".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.should_quit);
    }

    #[test]
    fn scroll_up_increases_offset() {
        let mut app = test_app();
        app.max_scroll = 100;
        app.scroll_up(5);
        assert_eq!(app.scroll_offset, 5);
        app.scroll_up(3);
        assert_eq!(app.scroll_offset, 8);
    }

    #[test]
    fn scroll_up_clamps_to_max_scroll() {
        let mut app = test_app();
        app.max_scroll = 10;
        app.scroll_up(15);
        assert_eq!(app.scroll_offset, 10);
    }

    #[test]
    fn scroll_down_decreases_offset_to_zero() {
        let mut app = test_app();
        app.scroll_offset = 10;
        app.scroll_down(3);
        assert_eq!(app.scroll_offset, 7);
        app.scroll_down(10);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn receiving_message_when_scrolled_up_increments_unread_count() {
        let mut app = test_app();
        app.scroll_offset = 5;
        app.receive_message(chat_msg("bot:ws", "hello"));
        assert_eq!(app.unread_count, 1);
        app.receive_message(chat_msg("bot:ws", "world"));
        assert_eq!(app.unread_count, 2);
    }

    #[test]
    fn scrolling_to_bottom_resets_unread_count() {
        let mut app = test_app();
        app.scroll_offset = 3;
        app.unread_count = 5;
        app.scroll_down(3);
        assert_eq!(app.scroll_offset, 0);
        assert_eq!(app.unread_count, 0);
    }

    #[test]
    fn receiving_message_at_bottom_does_not_increment_unread_count() {
        let mut app = test_app();
        assert_eq!(app.scroll_offset, 0);
        app.receive_message(chat_msg("bot:ws", "hello"));
        assert_eq!(app.unread_count, 0);
    }

    #[test]
    fn incoming_message_stays_at_bottom_when_at_bottom() {
        let mut app = test_app();
        assert_eq!(app.scroll_offset, 0);
        app.receive_message(chat_msg("bot:ws", "hello"));
        assert_eq!(app.scroll_offset, 0);
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn up_arrow_recalls_last_sent_message() {
        let mut app = test_app();
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
        let mut app = test_app();
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
        let mut app = test_app();
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
        let mut app = test_app();
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
        let mut app = test_app();
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
        let mut app = test_app();
        app.input = "/foo".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].from, "bpmux");
        assert!(app.messages[0].text.contains("Unknown command"));
        assert!(app.messages[0].text.contains("/foo"));
        assert!(app.messages[0].text.contains("?"));
    }

    #[test]
    fn incoming_message_preserves_viewport_when_scrolled_up() {
        let mut app = test_app();
        app.scroll_offset = 5;
        app.receive_message(chat_msg("bot:ws", "hello"));
        assert_eq!(app.scroll_offset, 6);
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn update_activity_adds_entry() {
        let mut app = test_app();
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        assert!(app.peer_activity.contains_key("dev:claude-code"));
        let (status, _) = &app.peer_activity["dev:claude-code"];
        assert_eq!(status, "working");
    }

    #[test]
    fn activity_message_does_not_appear_in_messages() {
        let mut app = test_app();
        app.receive_message(activity_msg("dev:claude-code", "working"));
        assert!(app.messages.is_empty());
        assert!(app.peer_activity.contains_key("dev:claude-code"));
    }

    #[test]
    fn debug_toggles_show_system_messages() {
        let mut app = test_app();
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
    fn to_single_recipient_returns_directed_message() {
        let mut app = test_app();
        app.input = "/to dev hello there".to_string();
        let msg = app.submit_message().unwrap();
        assert_eq!(msg.from, "ryan:human");
        assert_eq!(msg.text, "hello there");
        assert_eq!(msg.to, Some(vec!["dev".to_string()]));
    }

    #[test]
    fn to_csv_recipients_returns_directed_message() {
        let mut app = test_app();
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
        let mut app = test_app();
        app.input = "/to".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.messages[0].text.contains("Usage:"));
    }

    #[test]
    fn to_recipient_no_body_shows_usage() {
        let mut app = test_app();
        app.input = "/to dev".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.messages[0].text.contains("Usage:"));
    }

    #[test]
    fn tab_completes_participant_name_after_to() {
        let mut app = test_app();
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
        let mut app = test_app();
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        app.update_activity("pm:claude-code".to_string(), "idle".to_string());
        app.input = "/to dev,p".to_string();
        app.cursor_pos = app.input.len();
        assert!(app.tab_complete());
        assert_eq!(app.input, "/to dev,pm ");
    }

    #[test]
    fn tab_does_nothing_without_to_prefix() {
        let mut app = test_app();
        app.input = "hello".to_string();
        app.cursor_pos = app.input.len();
        assert!(!app.tab_complete());
        assert_eq!(app.input, "hello");
    }

    #[test]
    fn tab_does_nothing_in_body_portion() {
        let mut app = test_app();
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        app.input = "/to dev hel".to_string();
        app.cursor_pos = app.input.len();
        assert!(!app.tab_complete());
    }

    #[test]
    fn dm_increments_unread_and_sets_flash() {
        let mut app = test_app();
        app.receive_message(directed_msg("pm:claude-code", "hey ryan", vec!["ryan"]));
        assert_eq!(app.dm_unread_from.len(), 1);
        assert!(app.dm_flash_until.is_some());
    }

    #[test]
    fn broadcast_does_not_increment_dm_unread() {
        let mut app = test_app();
        app.receive_message(chat_msg("pm:claude-code", "hello all"));
        assert_eq!(app.dm_unread_from.len(), 0);
        assert!(app.dm_flash_until.is_none());
    }

    #[test]
    fn to_all_does_not_increment_dm_unread() {
        let mut app = test_app();
        app.receive_message(directed_msg("pm:claude-code", "hello", vec!["all"]));
        assert_eq!(app.dm_unread_from.len(), 0);
    }

    #[test]
    fn dm_to_other_does_not_increment_dm_unread() {
        let mut app = test_app();
        app.receive_message(directed_msg("pm:claude-code", "hey dev", vec!["dev"]));
        assert_eq!(app.dm_unread_from.len(), 0);
    }

    #[test]
    fn submit_clears_dm_unread() {
        let mut app = test_app();
        app.receive_message(directed_msg("pm:claude-code", "hey", vec!["ryan"]));
        assert_eq!(app.dm_unread_from.len(), 1);
        app.input = "got it".to_string();
        app.submit_message();
        assert_eq!(app.dm_unread_from.len(), 0);
        assert!(app.dm_flash_until.is_none());
    }

    #[test]
    fn broadcast_message_adds_channel() {
        let mut app = test_app();
        assert_eq!(app.active_channels.len(), 0);
        app.receive_message(chat_msg("dev:claude-code", "hello"));
        assert_eq!(app.active_channels.len(), 1);
        assert!(app.active_channels.contains("broadcast"));
    }

    #[test]
    fn directed_message_adds_new_channel() {
        let mut app = test_app();
        app.receive_message(chat_msg("dev:claude-code", "broadcast"));
        app.receive_message(directed_msg("dev:claude-code", "hey", vec!["ryan"]));
        assert_eq!(app.active_channels.len(), 2);
    }

    #[test]
    fn same_channel_does_not_duplicate() {
        let mut app = test_app();
        app.receive_message(directed_msg("dev:claude-code", "hey", vec!["ryan"]));
        app.receive_message(directed_msg("dev:claude-code", "also hey", vec!["ryan"]));
        assert_eq!(app.active_channels.len(), 1);
        assert!(app.active_channels.contains("dev"));
    }

    #[test]
    fn different_senders_create_different_channels() {
        let mut app = test_app();
        app.receive_message(directed_msg("dev:claude-code", "hey", vec!["ryan"]));
        app.receive_message(directed_msg("pm:claude-code", "also hey", vec!["ryan"]));
        assert_eq!(app.active_channels.len(), 2);
        assert!(app.active_channels.contains("dev"));
        assert!(app.active_channels.contains("pm"));
    }

    #[test]
    fn channel_name_broadcast() {
        let msg = chat_msg("dev:claude-code", "hi");
        assert_eq!(channel_name(&msg, "ryan"), "broadcast");
    }

    #[test]
    fn channel_name_to_all_is_broadcast() {
        let msg = directed_msg("dev:claude-code", "hi", vec!["all"]);
        assert_eq!(channel_name(&msg, "ryan"), "broadcast");
    }

    #[test]
    fn channel_name_dm_excludes_owner() {
        // dev sends to ryan — owner (ryan) is excluded, channel is just "dev"
        let msg = directed_msg("dev:claude-code", "hey", vec!["ryan"]);
        assert_eq!(channel_name(&msg, "ryan"), "dev");
    }

    #[test]
    fn channel_name_group_excludes_owner() {
        // pm sends to dev and ryan — owner excluded, channel is "dev+pm"
        let msg = directed_msg("pm:claude-code", "hey", vec!["dev", "ryan"]);
        assert_eq!(channel_name(&msg, "ryan"), "dev+pm");
    }

    #[test]
    fn channel_name_owner_sending_matches_channel() {
        // ryan sends to designer+dev+pm — same channel as when they talk
        let agent_msg = directed_msg("dev:claude-code", "hey", vec!["designer", "pm"]);
        let owner_msg = directed_msg("ryan:human", "hey", vec!["designer", "dev", "pm"]);
        assert_eq!(
            channel_name(&agent_msg, "ryan"),
            channel_name(&owner_msg, "ryan"),
            "owner sending into a channel should produce the same channel name"
        );
    }

    #[test]
    fn view_broadcast_sets_current_view() {
        let mut app = test_app();
        app.input = "/view broadcast".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert_eq!(app.current_view, Some("broadcast".to_string()));
    }

    #[test]
    fn view_all_clears_current_view() {
        let mut app = test_app();
        app.current_view = Some("broadcast".to_string());
        app.input = "/view all".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert_eq!(app.current_view, None);
    }

    #[test]
    fn view_known_channel_sets_current_view() {
        let mut app = test_app();
        app.active_channels.insert("dev\u{2194}ryan".to_string());
        app.input = "/view dev\u{2194}ryan".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert_eq!(app.current_view, Some("dev\u{2194}ryan".to_string()));
    }

    #[test]
    fn view_unknown_channel_shows_error() {
        let mut app = test_app();
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
        let mut app = test_app();
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
        let mut app = test_app();
        app.receive_message(activity_msg("dev:claude-code", "busy"));
        assert_eq!(app.active_channels.len(), 0);
    }

    #[test]
    fn popup_matches_filters_by_typed_prefix() {
        let mut app = test_app();
        app.input = "/v".to_string();
        let m = app.popup_matches();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].0, "/view");
    }

    #[test]
    fn popup_matches_returns_all_on_bare_slash() {
        let mut app = test_app();
        app.input = "/".to_string();
        assert_eq!(app.popup_matches().len(), SLASH_COMMANDS.len());
    }

    #[test]
    fn popup_matches_empty_after_space() {
        let mut app = test_app();
        app.input = "/to dev".to_string();
        assert!(app.popup_matches().is_empty());
    }

    #[test]
    fn popup_navigation_wraps_around() {
        let mut app = test_app();
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
        let mut app = test_app();
        app.input = "/t".to_string();
        app.popup_down(); // select /to
        assert!(app.popup_accept());
        assert_eq!(app.input, "/to ");
        assert_eq!(app.cursor_pos, 4);
        assert!(app.popup_selected.is_none());
    }

    #[test]
    fn popup_selected_resets_on_input_change() {
        let mut app = test_app();
        app.input = "/".to_string();
        app.cursor_pos = 1;
        app.popup_down();
        assert!(app.popup_selected.is_some());
        app.insert_char('t');
        assert!(app.popup_selected.is_none());
    }

    #[test]
    fn pin_adds_to_list() {
        let mut app = test_app();
        app.input = "/pin channels-as-views because filter-on-render".to_string();
        app.submit_message();
        assert_eq!(app.pins.len(), 1);
        assert_eq!(app.pins[0], "channels-as-views because filter-on-render");
        assert!(app.messages.last().unwrap().text.contains("Pinned #1"));
    }

    #[test]
    fn pin_max_five() {
        let mut app = test_app();
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
        let mut app = test_app();
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
        let mut app = test_app();
        app.input = "/pin something".to_string();
        app.submit_message();
        app.input = "/unpin 5".to_string();
        app.submit_message();
        assert_eq!(app.pins.len(), 1);
        assert!(app.messages.last().unwrap().text.contains("no pin #5"));
    }

    #[test]
    fn pins_command_lists_pins() {
        let mut app = test_app();
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
        let mut app = test_app();
        app.input = "/pins".to_string();
        app.submit_message();
        assert!(app.messages.last().unwrap().text.contains("No pins yet"));
    }

    #[test]
    fn pin_bare_shows_usage() {
        let mut app = test_app();
        app.input = "/pin".to_string();
        app.submit_message();
        assert!(app.messages.last().unwrap().text.contains("Usage:"));
    }

    #[test]
    fn unpin_bare_shows_usage() {
        let mut app = test_app();
        app.input = "/unpin".to_string();
        app.submit_message();
        assert!(app.messages.last().unwrap().text.contains("Usage:"));
    }

    // Sidebar tests

    #[test]
    fn sidebar_channels_starts_with_project_node() {
        let app = test_app();
        let channels = app.sidebar_channels();
        assert_eq!(channels[0], "test-session");
    }

    #[test]
    fn sidebar_channels_includes_active_channels() {
        let mut app = test_app();
        app.active_channels.insert("broadcast".to_string());
        app.active_channels.insert("dev↔human".to_string());
        let channels = app.sidebar_channels();
        assert!(channels.contains(&"broadcast".to_string()));
        assert!(channels.contains(&"dev↔human".to_string()));
    }

    #[test]
    fn sidebar_toggle_three_state_cycle() {
        let mut app = test_app();
        // Closed → open + focused
        assert!(!app.sidebar_open);
        app.toggle_sidebar();
        assert!(app.sidebar_open);
        assert!(app.sidebar_focused);
        // Open + focused → closed
        app.toggle_sidebar();
        assert!(!app.sidebar_open);
        assert!(!app.sidebar_focused);
        // Reopen, then unfocus (simulating channel select)
        app.toggle_sidebar();
        app.sidebar_focused = false; // simulate select unfocusing
        // Open + unfocused → refocused
        app.toggle_sidebar();
        assert!(app.sidebar_open);
        assert!(app.sidebar_focused);
    }

    #[test]
    fn sidebar_select_all_clears_view_and_target() {
        let mut app = test_app();
        app.current_view = Some("broadcast".to_string());
        app.send_target = Some("broadcast".to_string());
        app.sidebar_selected = 0; // project node is always first
        app.sidebar_select();
        assert!(app.current_view.is_none());
        assert!(app.send_target.is_none());
    }

    #[test]
    fn sidebar_select_channel_sets_view_and_target() {
        let mut app = test_app();
        app.active_channels.insert("broadcast".to_string());
        app.sidebar_selected = 1; // "broadcast" (sorted after project node)
        app.sidebar_select();
        assert_eq!(app.current_view, Some("broadcast".to_string()));
        assert_eq!(app.send_target, Some("broadcast".to_string()));
    }

    #[test]
    fn sidebar_select_clears_unread() {
        let mut app = test_app();
        app.active_channels.insert("broadcast".to_string());
        app.channel_unread.insert("broadcast".to_string(), 5);
        app.sidebar_selected = 1;
        app.sidebar_select();
        assert_eq!(app.channel_unread.get("broadcast"), None);
    }

    #[test]
    fn sidebar_navigation_wraps() {
        let mut app = test_app();
        app.active_channels.insert("broadcast".to_string());
        // channels: ["test-session", "broadcast"]
        app.sidebar_selected = 0;
        app.sidebar_up(); // wraps to last
        assert_eq!(app.sidebar_selected, 1);
        app.sidebar_down(); // wraps to first
        assert_eq!(app.sidebar_selected, 0);
    }

    #[test]
    fn sidebar_close_unfocuses_but_stays_open() {
        let mut app = test_app();
        app.toggle_sidebar();
        assert!(app.sidebar_open);
        assert!(app.sidebar_focused);
        app.sidebar_close(); // Esc — unfocuses but keeps sidebar open
        assert!(app.sidebar_open);
        assert!(!app.sidebar_focused);
    }

    #[test]
    fn sidebar_collapse_hides_channels() {
        let mut app = test_app();
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
        let mut app = test_app();
        app.active_channels.insert("broadcast".to_string());
        app.project_expanded = false;
        assert_eq!(app.sidebar_channels().len(), 1);
        app.sidebar_expand();
        assert!(app.project_expanded);
        assert_eq!(app.sidebar_channels().len(), 2);
    }

    #[test]
    fn sidebar_expanded_by_default() {
        let app = test_app();
        assert!(app.project_expanded);
    }

    #[test]
    fn send_target_directs_outgoing_messages() {
        let mut app = App::new("human".to_string(), "test-session".to_string(), vec![]);
        app.send_target = Some("dev↔human".to_string());
        app.input = "hello dev".to_string();
        app.cursor_pos = app.input.len();
        let msg = app.submit_message().unwrap();
        assert_eq!(msg.to, Some(vec!["dev".to_string()]));
    }

    #[test]
    fn no_send_target_broadcasts() {
        let mut app = App::new("human".to_string(), "test-session".to_string(), vec![]);
        app.input = "hello everyone".to_string();
        app.cursor_pos = app.input.len();
        let msg = app.submit_message().unwrap();
        assert!(msg.to.is_none());
    }

    #[test]
    fn channel_unread_increments_on_receive() {
        let mut app = App::new("human".to_string(), "test-session".to_string(), vec![]);
        app.current_view = Some("broadcast".to_string());
        app.receive_message(directed_msg("dev:claude-code", "dm message", vec!["human"]));
        assert_eq!(*app.channel_unread.get("dev").unwrap_or(&0), 1);
    }
}
