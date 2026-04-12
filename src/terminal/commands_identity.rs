//! `/identity` slash command — #190 identity layer view.
//!
//! Shows an agent's identity traits in the chat pane. Reads from
//! the identity store at `~/.bpmux/identity/{role}.json`.

use super::app::App;
use crate::session::Message;
use crate::session::identity::{default_identity_dir, load_identity};

impl App {
    pub(super) fn handle_identity(&mut self, args: String) -> Option<Message> {
        let role = if args.trim().is_empty() {
            // Default to showing all connected agents' identities.
            // For v1: show the queried role or list available.
            self.clear_input();
            self.show_all_identities();
            return None;
        } else {
            args.trim().to_string()
        };

        let base = match default_identity_dir() {
            Ok(b) => b,
            Err(e) => {
                self.system_message(format!("identity: {e}"));
                self.clear_input();
                return None;
            }
        };
        let index = load_identity(&base, &role);
        if index.traits.is_empty() {
            self.system_message(format!("identity ({role}): no traits defined yet."));
        } else {
            let mut text = format!("identity ({role}):\n");
            for t in &index.traits {
                text.push_str(&format!("  {} — {}\n", t.name, t.description));
            }
            self.system_message(text);
        }
        self.clear_input();
        None
    }

    fn show_all_identities(&mut self) {
        let base = match default_identity_dir() {
            Ok(b) => b,
            Err(e) => {
                self.system_message(format!("identity: {e}"));
                return;
            }
        };
        // List all identity files in the directory.
        let entries = match std::fs::read_dir(&base) {
            Ok(e) => e,
            Err(_) => {
                self.system_message(
                    "identity: no identity files yet. Use /identity <role> to query a specific role."
                        .to_string(),
                );
                return;
            }
        };
        let mut output = String::from("Identity traits:\n");
        let mut found = false;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Some(role) = path.file_stem().and_then(|s| s.to_str())
            {
                let index = load_identity(&base, role);
                if !index.traits.is_empty() {
                    found = true;
                    output.push_str(&format!("\n  {role}:\n"));
                    for t in &index.traits {
                        output.push_str(&format!("    {} — {}\n", t.name, t.description));
                    }
                }
            }
        }
        if !found {
            output.push_str("  (none defined yet)");
        }
        self.system_message(output);
    }
}
