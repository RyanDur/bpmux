use super::app::App;
use super::board;
use crate::session::{Message, MessageType};
use crate::transport::peer::P2pCommand;
use chrono::Utc;

pub(crate) const MAX_HISTORY: usize = 50;

impl App {
    pub fn submit_message(&mut self) -> Option<Message> {
        self.dm_unread_from.clear();
        self.dm_flash_until = None;
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return None;
        }

        if !self.search.results.is_empty() {
            if let Ok(n) = text.parse::<usize>() {
                self.expand_search_result(n);
                self.clear_input();
                return None;
            }
            if text == "next" {
                let next = self.search.selected.map(|i| i + 1).unwrap_or(0);
                self.expand_search_result(next);
                self.clear_input();
                return None;
            }
            if text == "prev" {
                let prev = self.search.selected.unwrap_or(1).saturating_sub(1);
                self.expand_search_result(prev);
                self.clear_input();
                return None;
            }
        }

        if text.starts_with('/') {
            let (cmd, args) = match text.find(' ') {
                Some(i) => (&text[..i], text[i + 1..].trim()),
                None => (text.as_str(), ""),
            };
            // #195: record command usage before dispatch.
            self.record_usage(cmd);
            return match cmd {
                "/who" => self.handle_who(),
                "/quit" => self.handle_quit(),
                "/help" => self.handle_help(),
                "/debug" => self.handle_debug(),
                "/status" => self.handle_status(text),
                "/to" => self.handle_to(args.to_string()),
                "/view" => self.handle_view(args.to_string()),
                "/search" => self.handle_search(args.to_string()),
                "/lessons" if !args.is_empty() => self.handle_peer_lessons(args.trim().to_string()),
                "/lessons" => self.handle_lessons(),
                "/board" => self.handle_board(),
                "/roadmap" => self.handle_roadmap(),
                "/session" => self.handle_session(),
                "/join" => self.handle_join(args.to_string()),
                "/accept" => self.handle_accept(args.trim().to_string()),
                "/share" => self.handle_share(args.to_string()),
                "/all" => self.handle_toggle_all(),
                "/tokens" => self.handle_tokens(),
                "/topic" => self.handle_topic(args.to_string()),
                "/focus" => self.handle_focus(args.to_string()),
                "/identity" => self.handle_identity(args.to_string()),
                _ => self.handle_unknown_command(text),
            };
        }

        self.send_chat_message(text)
    }

    fn handle_who(&mut self) -> Option<Message> {
        let mut participants = vec![format!("{} (you)", self.peer_name)];
        for (peer, (status, _)) in &self.peer_activity {
            participants.push(format!("{peer} [{status}]"));
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
        self.display.show_system_messages = !self.display.show_system_messages;
        let state = if self.display.show_system_messages {
            "on"
        } else {
            "off"
        };
        self.system_message(format!(
            "Debug mode {state}. System messages are now {}.",
            if self.display.show_system_messages {
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
            message_type: MessageType::Chat,
            to: None,
            addressee: None,
            explicit_privacy: false,
            multi_depth: None,
            topic: None,
        };
        self.push_message(msg.clone());
        self.clear_input();
        self.scroll.offset = 0;
        self.scroll.unread_count = 0;
        Some(msg)
    }

    fn handle_to(&mut self, rest: String) -> Option<Message> {
        let rest = rest.trim_start();
        let Some((recipients_str, body)) = rest.split_once(' ') else {
            return self.show_to_usage();
        };
        let body = body.trim().to_string();
        if body.is_empty() {
            return self.show_to_usage();
        }
        let recipients: Vec<String> = recipients_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if recipients.is_empty() {
            return self.show_to_usage();
        }
        // `/to` is user-explicit privacy intent. Flag it so /share can
        // refuse to propagate it and preserve the DM guarantee.
        let msg = Message {
            from: format!("{}:human", self.peer_name),
            text: body,
            ts: Utc::now(),
            message_type: MessageType::Chat,
            to: Some(recipients),
            addressee: None,
            explicit_privacy: true,
            multi_depth: None,
            topic: None,
        };
        self.push_message(msg.clone());
        self.clear_input();
        self.scroll.offset = 0;
        self.scroll.unread_count = 0;
        if self.input_history.len() >= MAX_HISTORY {
            self.input_history.remove(0);
        }
        self.input_history.push(self.input.clone());
        self.history_index = None;
        Some(msg)
    }

    fn show_to_usage(&mut self) -> Option<Message> {
        self.system_message(
            "Usage: /to <recipients> <message>. Example: /to dev,pm hello".to_string(),
        );
        self.clear_input();
        None
    }

    fn handle_view(&mut self, arg: String) -> Option<Message> {
        let arg = arg.trim();
        if arg.is_empty() {
            // List available channels (the bare-form behavior)
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
            return None;
        }
        if arg == "all" {
            self.current_view = None;
            self.mark_filters_dirty();
            self.scroll.offset = 0;
            self.system_message("View set to all channels.".to_string());
            self.clear_input();
            return None;
        }
        if arg == "broadcast" {
            self.current_view = Some("broadcast".to_string());
            self.mark_filters_dirty();
            self.scroll.offset = 0;
            self.system_message("View set to broadcast.".to_string());
            self.clear_input();
            return None;
        }
        // Check if arg matches a known channel
        if self.active_channels.contains(arg) {
            self.current_view = Some(arg.to_string());
            self.mark_filters_dirty();
            self.scroll.offset = 0;
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

    fn handle_roadmap(&mut self) -> Option<Message> {
        let roadmap_text = match board::fetch_roadmap() {
            Ok(text) => text,
            Err(e) => format!("Roadmap unavailable: {e}"),
        };
        self.system_message(roadmap_text);
        self.clear_input();
        None
    }

    fn handle_board(&mut self) -> Option<Message> {
        match board::fetch_board_stories() {
            Ok(stories) => {
                self.board.stories = stories;
                self.board.selected = 0;
                self.board.detail = None;
                // #196: open in project panel (split view) by default.
                // The full-screen board is still accessible via the
                // board_detail navigation.
                self.project.show = true;
            }
            Err(e) => {
                self.system_message(format!(
                    "Board unavailable: {e}\n\
                     Ensure `gh` CLI is installed and authenticated (`gh auth login`)."
                ));
            }
        }
        self.clear_input();
        None
    }

    fn handle_session(&mut self) -> Option<Message> {
        self.system_message("Creating P2P session... (gathering ICE candidates)".to_string());
        self.p2p_command = Some(P2pCommand::CreateSession);
        self.clear_input();
        None
    }

    fn handle_join(&mut self, args: String) -> Option<Message> {
        let args = args.trim();
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if args.is_empty() || parts.len() < 2 {
            self.system_message("Usage: /join <passphrase> <offer-code>".to_string());
            self.clear_input();
            return None;
        }
        let passphrase = parts[0].to_string();
        let offer_code = parts[1].trim().to_string();
        self.system_message(
            "Joining session... (decrypting offer, gathering ICE candidates)".to_string(),
        );
        self.p2p_command = Some(P2pCommand::JoinSession {
            passphrase,
            offer_code,
        });
        self.clear_input();
        None
    }

    fn handle_accept(&mut self, answer_code: String) -> Option<Message> {
        let answer_code = answer_code.trim().to_string();
        if answer_code.is_empty() {
            self.system_message("Usage: /accept <answer-code>".to_string());
            self.clear_input();
            return None;
        }
        self.system_message("Accepting answer... (establishing connection)".to_string());
        self.p2p_command = Some(P2pCommand::AcceptAnswer { answer_code });
        self.clear_input();
        None
    }

    /// `/all` — toggle the classifier-driven view filter (#161).
    /// Default state: filter ON, Routine + bpmux-nudges hidden from your
    /// pane. `/all` once flips to "show everything" mode. `/all` again
    /// flips back to filtered.
    ///
    /// No system_message — the status bar already shows "N hidden" /
    /// "showing all" so the chat-pane line was redundant feedback (the
    /// "noise about a noise filter" anti-pattern designer flagged on
    /// 2026-04-10). Removing the source beats adding a classifier rule.
    fn handle_toggle_all(&mut self) -> Option<Message> {
        self.display.show_all_classes = !self.display.show_all_classes;
        self.mark_filters_dirty();
        self.clear_input();
        None
    }

    fn handle_unknown_command(&mut self, text: String) -> Option<Message> {
        self.system_message(format!(
            "Unknown command: {}. Type /help or press ? for the command list.",
            text
        ));
        self.clear_input();
        None
    }

    fn send_chat_message(&mut self, text: String) -> Option<Message> {
        // Parse `@role ` prefix as an attention signal (#158).
        // `@pm what about X?` sets addressee="pm" and strips the prefix.
        // Distinct from `/to pm "..."` which sets the privacy filter — the
        // `@role` prefix is advisory addressing on broadcast messages.
        let (addressee, text) = parse_addressee_prefix(&text);
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
                vec![ch.clone()]
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
            message_type: MessageType::Chat,
            to,
            addressee,
            explicit_privacy: false,
            multi_depth: None,
            topic: if self.topic.active == "general" {
                None
            } else {
                Some(self.topic.active.clone())
            },
        };
        self.push_message(msg.clone());
        self.clear_input();
        self.scroll.offset = 0;
        self.scroll.unread_count = 0;
        if self.input_history.len() >= MAX_HISTORY {
            self.input_history.remove(0);
        }
        self.input_history.push(msg.text.clone());
        self.history_index = None;
        Some(msg)
    }
}

/// Parse an optional `@role ` prefix from chat input (#158). Returns
/// `(addressee, remaining_text)`. Only matches a leading `@` followed by
/// alphanumeric/hyphen characters and a single space — conservative on
/// purpose so `foo @mention bar` in the body doesn't accidentally set an
/// addressee. Returns `(None, original_text)` if no prefix is present.
///
/// Accepts `@all` as an explicit broadcast addressee (sets
/// addressee="all") because the spec lists "all" alongside role names.
pub(super) fn parse_addressee_prefix(text: &str) -> (Option<String>, String) {
    if !text.starts_with('@') {
        return (None, text.to_string());
    }
    // Walk past '@' and collect alphanumeric/hyphen chars until whitespace.
    let mut chars = text.char_indices();
    chars.next(); // consume '@'
    let mut end = 1;
    for (i, c) in chars {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            end = i + c.len_utf8();
            continue;
        }
        break;
    }
    // Must have at least one char after '@' AND be followed by whitespace
    // to count as a prefix. Bare `@` or `@role` with no body is passthrough.
    let role = &text[1..end];
    if role.is_empty() {
        return (None, text.to_string());
    }
    let rest = &text[end..];
    let rest_trimmed = rest.trim_start();
    if rest.len() == rest_trimmed.len() {
        // No whitespace after `@role` — treat as normal text, not a prefix.
        return (None, text.to_string());
    }
    (Some(role.to_string()), rest_trimmed.to_string())
}
