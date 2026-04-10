use super::app::App;
use super::board;
use crate::session::{Message, MessageType};
use crate::transport::peer::P2pCommand;
use chrono::Utc;

pub(super) const MAX_HISTORY: usize = 50;

impl App {
    pub fn submit_message(&mut self) -> Option<Message> {
        self.dm_unread_from.clear();
        self.dm_flash_until = None;
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return None;
        }

        if !self.search_results.is_empty() {
            if let Ok(n) = text.parse::<usize>() {
                self.expand_search_result(n);
                self.clear_input();
                return None;
            }
            if text == "next" {
                let next = self.search_selected.map(|i| i + 1).unwrap_or(0);
                self.expand_search_result(next);
                self.clear_input();
                return None;
            }
            if text == "prev" {
                let prev = self.search_selected.unwrap_or(1).saturating_sub(1);
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
            return match cmd {
                "/who" => self.handle_who(),
                "/quit" => self.handle_quit(),
                "/debug" => self.handle_debug(),
                "/status" => self.handle_status(text),
                "/to" if !args.is_empty() => self.handle_to_with_args(args.to_string()),
                "/to" => self.handle_to_bare(),
                "/view" if !args.is_empty() => self.handle_view_with_arg(args.to_string()),
                "/view" => self.handle_view_bare(),
                "/search" if !args.is_empty() => self.handle_search(args.to_string()),
                "/search" => self.handle_search_bare(),
                "/lessons" if !args.is_empty() => self.handle_peer_lessons(args.trim().to_string()),
                "/lessons" => self.handle_lessons(),
                "/board" => self.handle_board(),
                "/roadmap" => self.handle_roadmap(),
                "/session" => self.handle_session(),
                "/join" if !args.is_empty() => self.handle_join(args.to_string()),
                "/join" => self.handle_join_bare(),
                "/accept" if !args.is_empty() => self.handle_accept(args.trim().to_string()),
                "/accept" => self.handle_accept_bare(),
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
            message_type: MessageType::Chat,
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
        let msg = Message {
            from: format!("{}:human", self.peer_name),
            text: body,
            ts: Utc::now(),
            message_type: MessageType::Chat,
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
        Some(msg)
    }

    fn handle_to_bare(&mut self) -> Option<Message> {
        self.show_to_usage()
    }

    fn show_to_usage(&mut self) -> Option<Message> {
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
        let board_text = match board::fetch_board() {
            Ok(text) => text,
            Err(e) => format!(
                "Board unavailable: {e}\nEnsure `gh` CLI is installed and authenticated (`gh auth login`)."
            ),
        };
        self.system_message(board_text);
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
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.len() < 2 {
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

    fn handle_join_bare(&mut self) -> Option<Message> {
        self.system_message("Usage: /join <passphrase> <offer-code>".to_string());
        self.clear_input();
        None
    }

    fn handle_accept(&mut self, answer_code: String) -> Option<Message> {
        self.system_message("Accepting answer... (establishing connection)".to_string());
        self.p2p_command = Some(P2pCommand::AcceptAnswer { answer_code });
        self.clear_input();
        None
    }

    fn handle_accept_bare(&mut self) -> Option<Message> {
        self.system_message("Usage: /accept <answer-code>".to_string());
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
}
