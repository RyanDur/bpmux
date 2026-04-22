//! `/identity` slash command — #190 identity layer view.
//!
//! Shows an agent's identity in the chat pane. Reads from the
//! identity store at `~/.bpmux/identity/{role}.json`. Schema is
//! values→principles (#269).

use crate::session::Message;
use crate::session::identity::{IdentityIndex, default_identity_dir, load_identity};
use crate::terminal::app::App;

impl App {
    pub(in crate::terminal) fn handle_identity(&mut self, args: String) -> Option<Message> {
        let role = if args.trim().is_empty() {
            self.clear_input();
            self.show_all_identities();
            return None;
        } else {
            args.trim().to_string()
        };

        let base = match default_identity_dir() {
            Ok(b) => b,
            Err(_) => {
                self.system_message("Unable to load identity.".to_string());
                self.clear_input();
                return None;
            }
        };
        let index = load_identity(&base, &role);
        if index.values.is_empty() {
            self.system_message(format!("identity ({role}): no values defined yet."));
        } else {
            self.system_message(format_identity(&role, &index));
        }
        self.clear_input();
        None
    }

    fn show_all_identities(&mut self) {
        let base = match default_identity_dir() {
            Ok(b) => b,
            Err(_) => {
                self.system_message("Unable to load identity.".to_string());
                return;
            }
        };
        let entries = match std::fs::read_dir(&base) {
            Ok(e) => e,
            Err(_) => {
                self.system_message("No identity files found.".to_string());
                return;
            }
        };
        let mut output = String::from("Identity:\n");
        let mut found = false;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Some(role) = path.file_stem().and_then(|s| s.to_str())
            {
                let index = load_identity(&base, role);
                if !index.values.is_empty() {
                    found = true;
                    output.push('\n');
                    output.push_str(&format_identity(role, &index));
                }
            }
        }
        if !found {
            output.push_str("  (none defined yet)");
        }
        self.system_message(output);
    }
}

fn format_identity(role: &str, index: &IdentityIndex) -> String {
    let mut out = format!("identity ({role}):\n");
    for value in &index.values {
        out.push_str(&format!("  {}: {}\n", value.name, value.description));
        for principle in &value.principles {
            out.push_str(&format!("    - {principle}\n"));
        }
    }
    out
}
