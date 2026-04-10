//! `/help` slash command + shared command-list formatter (#164).
//!
//! Extracted from `commands.rs` to keep that file under the 400-line
//! soft limit. The formatter is shared by:
//!
//! - `/help` chat-pane output (`handle_help` here)
//! - The `?` keybind help overlay (`ui::render_help_overlay`)
//!
//! Both consumers source from `input::SLASH_COMMANDS` so the chat-pane
//! and overlay forms can never drift — adding a command means adding
//! one line to that constant and nothing else.

use super::app::App;
use super::input::SLASH_COMMANDS;
use crate::session::Message;

impl App {
    /// `/help` (#164) — print the slash-command list inline in the chat
    /// pane. Sources from `SLASH_COMMANDS` so this list and the `?`
    /// overlay can never drift again. Uses the single-column form
    /// because chat-pane width is generous and full descriptions are
    /// the whole point of `/help` (vs. the overlay's compact form).
    pub(super) fn handle_help(&mut self) -> Option<Message> {
        self.system_message(format_command_list(1));
        self.clear_input();
        None
    }
}

/// Build the slash-command list text from `SLASH_COMMANDS`. Used by
/// `/help` (chat-pane) and `?` keybind (overlay) so they can never drift
/// (#164).
///
/// `columns` controls the layout: `1` produces a single column with the
/// full description per command (chat-pane form, more readable), `2`
/// produces a compact two-column form that fits in the 24-row help
/// overlay alongside the keys section. The two consumers have different
/// space budgets so the same data is rendered two ways.
pub(super) fn format_command_list(columns: usize) -> String {
    let mut out = String::from("Commands:\n");
    let widest = SLASH_COMMANDS
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(0);
    if columns <= 1 {
        for (name, desc) in SLASH_COMMANDS {
            out.push_str(&format!("  {name:<widest$}  {desc}\n"));
        }
    } else {
        // Two-column layout: drop the descriptions (the names alone are
        // discoverable; users wanting full descriptions can type `/help`
        // for the chat-pane form). Lay commands out left-to-right,
        // top-to-bottom in groups of `columns`.
        let col_width = widest + 2;
        for chunk in SLASH_COMMANDS.chunks(columns) {
            out.push_str("  ");
            for (i, (name, _)) in chunk.iter().enumerate() {
                if i + 1 < chunk.len() {
                    out.push_str(&format!("{name:<col_width$}"));
                } else {
                    out.push_str(name);
                }
            }
            out.push('\n');
        }
    }
    // Strip the trailing newline so the system_message doesn't render
    // an empty line at the bottom.
    out.pop();
    out
}
