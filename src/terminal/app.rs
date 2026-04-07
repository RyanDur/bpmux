use crate::session::Message;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

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
        let sender = short_peer_name(&msg.from).to_string();

        match msg.message_type.as_deref() {
            Some("activity") => {
                self.update_activity(sender, msg.text);
                return;
            }
            Some("system") => {
                if let Some(peer) = msg.text.strip_suffix(" joined") {
                    self.update_activity(short_peer_name(peer).to_string(), "available".into());
                } else if let Some(peer) = msg.text.strip_suffix(" left") {
                    self.peer_activity.remove(short_peer_name(peer));
                }
            }
            _ => {
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
                        self.dm_flash_until =
                            Some(Instant::now() + std::time::Duration::from_millis(750));
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::terminal::commands::MAX_HISTORY;
    use crate::terminal::input::SLASH_COMMANDS;

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
        app.update_activity("dev".to_string(), "working".to_string());
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
        app.update_activity("dev".to_string(), "working".to_string());
        assert!(app.peer_activity.contains_key("dev"));
        let (status, _) = &app.peer_activity["dev"];
        assert_eq!(status, "working");
    }

    #[test]
    fn activity_message_does_not_appear_in_messages() {
        let mut app = test_app();
        app.receive_message(activity_msg("dev:claude-code", "working"));
        assert!(app.messages.is_empty());
        assert!(app.peer_activity.contains_key("dev"));
    }

    #[test]
    fn peer_activity_not_rebuilt_from_log() {
        let messages = vec![
            system_msg("bpmux", "dev:ws joined"),
            activity_msg("dev:ws", "busy"),
            system_msg("bpmux", "pm:ws joined"),
        ];
        let app = App::new("ryan".to_string(), "test-session".to_string(), messages);
        assert!(
            app.peer_activity.is_empty(),
            "peer_activity should be empty on startup — only live connections populate it"
        );
    }

    #[test]
    fn live_join_uses_short_name() {
        let mut app = test_app();
        app.receive_message(system_msg("bpmux", "dev:claude-code joined"));
        app.receive_message(system_msg("bpmux", "dev:ws joined"));
        assert_eq!(
            app.peer_activity.len(),
            1,
            "same role via different transports should dedupe"
        );
        assert!(app.peer_activity.contains_key("dev"));
    }

    #[test]
    fn live_leave_removes_by_short_name() {
        let mut app = test_app();
        app.receive_message(system_msg("bpmux", "dev:claude-code joined"));
        assert!(app.peer_activity.contains_key("dev"));
        app.receive_message(system_msg("bpmux", "dev:ws left"));
        assert!(
            !app.peer_activity.contains_key("dev"),
            "leave from any transport should remove"
        );
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
        app.active_channels.insert("dev\u{2194}human".to_string());
        let channels = app.sidebar_channels();
        assert!(channels.contains(&"broadcast".to_string()));
        assert!(channels.contains(&"dev\u{2194}human".to_string()));
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
        app.send_target = Some("dev\u{2194}human".to_string());
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

    #[test]
    fn roadmap_command_adds_system_message() {
        // fetch_roadmap uses git rev-parse which works when tests run from the repo
        let mut app = test_app();
        app.input = "/roadmap".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        let last = app.messages.last().expect("should have a message");
        assert_eq!(last.from, "bpmux");
        // Either shows roadmap content or an unavailable message
        assert!(last.text.contains("bpmux Roadmap") || last.text.contains("Roadmap unavailable"),);
    }

    #[test]
    fn roadmap_command_clears_input() {
        let mut app = test_app();
        app.input = "/roadmap".to_string();
        app.submit_message();
        assert!(app.input.is_empty());
    }

    #[test]
    fn toggle_expands_long_system_message() {
        let mut app = test_app();
        // Add a long message from "bpmux" (like /roadmap output)
        let long_text = "x".repeat(1000); // ~13 wrapped lines at width 78
        app.messages.push(Message {
            from: "bpmux".into(),
            text: long_text,
            ts: chrono::Utc::now(),
            message_type: None,
            to: None,
        });
        let key = app.messages.last().unwrap().ts.to_rfc3339();
        assert!(
            !app.expanded_messages.contains(&key),
            "should not be expanded initially"
        );
        app.toggle_last_truncated();
        assert!(
            app.expanded_messages.contains(&key),
            "should be expanded after toggle"
        );
        app.toggle_last_truncated();
        assert!(
            !app.expanded_messages.contains(&key),
            "should collapse after second toggle"
        );
    }
}
