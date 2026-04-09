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
        if let Some(query) = text.strip_prefix("/search ") {
            return self.handle_search(query.to_string());
        }
        if text == "/search" {
            return self.handle_search_bare();
        }
        if text == "/lessons" {
            return self.handle_lessons();
        }
        if let Some(peer) = text.strip_prefix("/lessons ") {
            return self.handle_peer_lessons(peer.trim().to_string());
        }
        if text == "/board" {
            return self.handle_board();
        }
        if text == "/roadmap" {
            return self.handle_roadmap();
        }
        if text == "/session" {
            return self.handle_session();
        }
        if let Some(args) = text.strip_prefix("/join ") {
            return self.handle_join(args.to_string());
        }
        if text == "/join" {
            return self.handle_join_bare();
        }
        if let Some(code) = text.strip_prefix("/accept ") {
            return self.handle_accept(code.trim().to_string());
        }
        if text == "/accept" {
            return self.handle_accept_bare();
        }
        if text.starts_with('/') {
            return self.handle_unknown_command(text);
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

    fn handle_search(&mut self, query: String) -> Option<Message> {
        let parts: Vec<&str> = query.splitn(2, ' ').collect();
        let (message_class, keyword) = match parts[0] {
            "corrections" | "experiences" => (
                Some(crate::session::MessageClass::Experience(
                    crate::session::classifier::ExperienceKind::Correction,
                )),
                parts.get(1).unwrap_or(&"").to_string(),
            ),
            "decisions" => (
                Some(crate::session::MessageClass::Decision),
                parts.get(1).unwrap_or(&"").to_string(),
            ),
            _ => (None, query.clone()),
        };
        let filters = crate::session::SearchFilters {
            query: keyword,
            sender: None,
            after: None,
            before: None,
            channel: None,
            message_class,
            owner_name: self.peer_name.clone(),
        };
        let results = crate::session::search(&self.messages, &filters);
        if results.is_empty() {
            self.system_message(format!("No results for \"{query}\"."));
            self.search_results.clear();
            self.search_selected = None;
        } else {
            let mut text = format!(
                "── Search: \"{query}\" ({} results) ── type a number to expand ──\n",
                results.len()
            );
            for (i, result) in results.iter().enumerate() {
                let class_label = format!("{:?}", result.message_class).to_lowercase();
                let ts = result.messages[0].ts.format("%Y-%m-%d %H:%M");
                let sender = result.messages[result.match_index]
                    .from
                    .split(':')
                    .next()
                    .unwrap_or("?");
                let preview: String = result.messages[result.match_index]
                    .text
                    .chars()
                    .take(60)
                    .collect();
                text.push_str(&format!(
                    "  [{i}] [{class_label}] {ts} {sender}: {preview}\n"
                ));
            }
            text.push_str("── type number to expand, /search for new query ──");
            self.system_message(text);
            self.search_results = results;
            self.search_selected = None;
        }
        self.clear_input();
        None
    }

    fn handle_lessons(&mut self) -> Option<Message> {
        let mut index = crate::session::lessons::build_index(&self.messages, &self.peer_name);
        if let Some(dir) = &self.session_dir {
            let saved = crate::session::lessons::load_index(dir);
            index.lessons = saved.lessons;
        }
        let groups = crate::session::lessons::group_by_topic(&index);
        if groups.is_empty() {
            self.system_message("No experiences indexed yet.".to_string());
        } else {
            let mut text = format!("── Team Lessons ({} topics) ──\n", groups.len());
            for group in &groups {
                let experience_count = group.experiences.len();
                let lesson_count = group.lessons.len();
                text.push_str(&format!(
                    "\n  {} ({} experience{}, {} lesson{})\n",
                    group.topic,
                    experience_count,
                    if experience_count == 1 { "" } else { "s" },
                    lesson_count,
                    if lesson_count == 1 { "" } else { "s" },
                ));
                if !group.experiences.is_empty() {
                    text.push_str("    Corrections:\n");
                    for c in &group.experiences {
                        text.push_str(&format!(
                            "      {}: {}\n",
                            &c.timestamp[..10],
                            c.text_preview
                        ));
                    }
                }
                if !group.lessons.is_empty() {
                    text.push_str("    Lessons:\n");
                    for l in &group.lessons {
                        text.push_str(&format!("      {}: {}\n", l.author_role, l.lesson));
                    }
                }
            }
            text.push_str("\n── /search experiences <topic> for details ──");
            self.system_message(text);
        }
        if let Some(dir) = &self.session_dir {
            let _ = crate::session::lessons::save_index(dir, &index);
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

    fn handle_peer_lessons(&mut self, peer_name: String) -> Option<Message> {
        let Some(dir) = &self.session_dir else {
            self.system_message("No session directory available.".to_string());
            self.clear_input();
            return None;
        };

        let available_peers = crate::session::lessons::list_peer_names(dir);
        if !available_peers.contains(&peer_name) {
            if available_peers.is_empty() {
                self.system_message(
                    "No peer lesson indexes available. Connect to a peer first.".to_string(),
                );
            } else {
                self.system_message(format!(
                    "Peer \"{peer_name}\" not found. Available: {}",
                    available_peers.join(", ")
                ));
            }
            self.clear_input();
            return None;
        }

        let index = crate::session::lessons::load_peer_index(dir, &peer_name);
        let groups = crate::session::lessons::group_by_topic(&index);
        if groups.is_empty() {
            self.system_message(format!("No lessons from peer \"{peer_name}\"."));
        } else {
            let mut text = format!(
                "── Lessons from {peer_name} ({} topics, read-only) ──\n",
                groups.len()
            );
            for group in &groups {
                let experience_count = group.experiences.len();
                let lesson_count = group.lessons.len();
                text.push_str(&format!(
                    "\n  {} ({} experience{}, {} lesson{})\n",
                    group.topic,
                    experience_count,
                    if experience_count == 1 { "" } else { "s" },
                    lesson_count,
                    if lesson_count == 1 { "" } else { "s" },
                ));
                if !group.lessons.is_empty() {
                    for l in &group.lessons {
                        text.push_str(&format!("    {}: {}\n", l.author_role, l.lesson));
                    }
                }
            }
            text.push_str("\n── read-only — use save_lesson to internalize ──");
            self.system_message(text);
        }
        self.clear_input();
        None
    }

    pub(super) fn expand_search_result(&mut self, index: usize) {
        if let Some(result) = self.search_results.get(index) {
            let mut text = format!("── Result [{index}] ({:?}) ──\n", result.message_class);
            for (i, msg) in result.messages.iter().enumerate() {
                let sender = msg.from.split(':').next().unwrap_or(&msg.from);
                let ts = msg.ts.format("%H:%M:%S");
                let marker = if i == result.match_index { " ◀" } else { "" };
                text.push_str(&format!("  [{ts}] {sender}: {}{marker}\n", msg.text));
            }
            text.push_str("── /search for new query ──");
            self.system_message(text);
            self.search_selected = Some(index);
        }
    }

    fn handle_search_bare(&mut self) -> Option<Message> {
        self.system_message("Usage: /search <query>. Example: /search role boundaries".to_string());
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
