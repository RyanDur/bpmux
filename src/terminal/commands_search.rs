//! Search and lesson-index command handlers extracted from commands.rs.
//!
//! These live on the same `App` type as the other command handlers but in a
//! separate file so commands.rs stays under the 400-line quality gate.

use super::app::App;
use crate::session::Message;

impl App {
    pub(super) fn handle_search(&mut self, query: String) -> Option<Message> {
        let query = query.trim().to_string();
        if query.is_empty() {
            self.system_message(
                "Usage: /search <query>. Example: /search role boundaries".to_string(),
            );
            self.clear_input();
            return None;
        }
        let parts: Vec<&str> = query.splitn(2, ' ').collect();
        let (message_class, insight_only, keyword) = match parts[0] {
            "corrections" | "experiences" => (
                Some(crate::session::MessageClass::Experience(
                    crate::session::classifier::ExperienceKind::Correction,
                )),
                false,
                parts.get(1).unwrap_or(&"").to_string(),
            ),
            "decisions" => (
                Some(crate::session::MessageClass::Decision),
                false,
                parts.get(1).unwrap_or(&"").to_string(),
            ),
            "insights" => (None, true, parts.get(1).unwrap_or(&"").to_string()),
            _ => (None, false, query.clone()),
        };
        let filters = crate::session::SearchFilters {
            query: keyword,
            sender: None,
            after: None,
            before: None,
            channel: None,
            message_class,
            insight_only,
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

    pub(super) fn handle_lessons(&mut self) -> Option<Message> {
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

    pub(super) fn handle_peer_lessons(&mut self, peer_name: String) -> Option<Message> {
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
}
