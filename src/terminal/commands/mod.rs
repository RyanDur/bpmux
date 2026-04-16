mod board;
mod chat_message;
mod debug;
pub(crate) mod focus;
pub(crate) mod help;
mod identity;
mod p2p;
mod quit;
mod roadmap;
mod search;
mod share;
mod status;
mod to;
mod toggle_all;
pub(crate) mod tokens;
mod topic;
mod who;

#[cfg(test)]
mod tests;

use super::app::App;
use crate::session::Message;

pub(crate) const MAX_HISTORY: usize = 50;

impl App {
    pub fn submit_message(&mut self) -> Option<Message> {
        self.peers.dm_unread_from.clear();
        self.peers.dm_flash_until = None;
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
            // Record command usage before dispatch.
            self.record_usage(cmd);
            return match super::input::find_slash_command(cmd) {
                Some(entry) => (entry.handler)(self, args),
                None => self.handle_unknown_command(text),
            };
        }

        self.send_chat_message(text)
    }

    /// Shared view-switch core. Both `/view` (typed) and
    /// `sidebar_select` (clicked) route through this so the filter
    /// state stays in lockstep — no two-implementation drift when we
    /// later delete the slash command. Context-specific behavior
    /// (system messages, send_target updates, unread clearing) stays
    /// in each entry point.
    pub(crate) fn switch_view_to(&mut self, channel: Option<String>) {
        self.channels.current_view = channel;
        self.mark_filters_dirty();
        self.scroll.offset = 0;
    }

    fn handle_unknown_command(&mut self, text: String) -> Option<Message> {
        self.system_message(format!(
            "Unknown command: {}. Type /help or press ? for the command list.",
            text
        ));
        self.clear_input();
        None
    }
}
