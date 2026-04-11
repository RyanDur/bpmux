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
/// full (long) description per command — the chat-pane form, where width
/// is generous. `2` produces a compact two-column form with short
/// descriptions — the overlay form, where width is tight but names alone
/// aren't enough to teach the user what each command does. Both forms
/// include descriptions; only the LENGTH of the description varies by
/// surface. This was the load-bearing fix in the #164 re-delivery — the
/// original shipped `2` form dropped descriptions entirely, which Ryan
/// correctly rejected as a regression.
pub(super) fn format_command_list(columns: usize) -> String {
    let mut out = String::from("Commands:\n");
    let widest_name = SLASH_COMMANDS
        .iter()
        .map(|(name, _, _)| name.len())
        .max()
        .unwrap_or(0);
    if columns <= 1 {
        // Single-column chat-pane form: full long descriptions.
        for (name, _, long) in SLASH_COMMANDS {
            out.push_str(&format!("  {name:<widest_name$}  {long}\n"));
        }
    } else {
        // Two-column compact overlay form: short descriptions alongside
        // names so users can still learn what each command does without
        // dismissing the overlay. Each entry is
        // `{name: <widest_name> } {short_desc: <widest_short>}` and
        // columns are laid out left-to-right, top-to-bottom in groups
        // of `columns`. The overlay width (set in `ui.rs`) must be
        // large enough to fit `(name_col + 1 + short_col) * columns +
        // leading_indent + inter_col_gaps`.
        let widest_short = SLASH_COMMANDS
            .iter()
            .map(|(_, short, _)| short.len())
            .max()
            .unwrap_or(0);
        let entry_width = widest_name + 1 + widest_short; // `name short`
        let gap = 2; // spaces between columns
        for chunk in SLASH_COMMANDS.chunks(columns) {
            out.push_str("  ");
            for (i, (name, short, _)) in chunk.iter().enumerate() {
                let entry = format!("{name:<widest_name$} {short:<widest_short$}");
                if i + 1 < chunk.len() {
                    out.push_str(&format!("{entry:<width$}", width = entry_width + gap));
                } else {
                    out.push_str(&entry);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the #164 rejection — the 2-column overlay form
    /// must include short descriptions alongside command names, not just
    /// names alone. The original #164 delivery shipped with names-only in
    /// the 2-column branch, which produced an overlay that showed
    /// `/to /share /view` etc. with no hint at what each command did.
    /// Ryan rejected that as a regression against the pre-#164 overlay
    /// (which had descriptions in a more compact layout) and re-opened
    /// the story for a fix.
    ///
    /// This test asserts that at least one description word is present in
    /// the 2-column output. Any short description verb (`send`, `propagate`,
    /// `filter`, etc.) is acceptable — the test is tolerant of the specific
    /// phrasing so the green implementation can choose short descriptions
    /// that fit the 52-char overlay width budget without locking in a
    /// particular form.
    #[test]
    fn overlay_form_includes_short_descriptions_beside_names() {
        let output = format_command_list(2);
        // Sanity: command names are still present in the 2-column form.
        assert!(
            output.contains("/to"),
            "expected /to in 2-column output, got:\n{output}"
        );
        assert!(
            output.contains("/share"),
            "expected /share in 2-column output, got:\n{output}"
        );
        // Regression guard: at least one description token from the
        // canonical SLASH_COMMANDS descriptions must be present. A
        // names-only output (the bug state) has none of these words.
        //
        // Tokens chosen carefully to avoid collision with command names
        // themselves — e.g., "search" would false-positive because
        // `/search` contains "search" as part of the slash command name.
        // The words below appear in descriptions but NOT in any slash
        // command name: "propagate" only in /share's description,
        // "toggle" only in /all and /debug descriptions, "filter" only
        // in /all's description, "recipients" only in /to's description.
        let has_description = output.contains("propagate")
            || output.contains("toggle")
            || output.contains("filter")
            || output.contains("recipients");
        assert!(
            has_description,
            "2-column overlay form must include short descriptions beside names — \
             the names-only output fails to teach users what each command does. \
             expected at least one of: propagate, toggle, filter, recipients. got:\n{output}"
        );
    }

    /// Single-column form should still carry the full descriptions from
    /// `SLASH_COMMANDS` (unchanged by the #164 rework). Included here so
    /// the rework doesn't accidentally break the chat-pane `/help` output
    /// while fixing the overlay.
    #[test]
    fn chat_pane_form_still_carries_long_descriptions() {
        let output = format_command_list(1);
        assert!(output.contains("/to"));
        // `send to recipients` is from /to's long description in SLASH_COMMANDS.
        assert!(
            output.contains("send to recipients"),
            "1-column chat-pane output must carry the long description from SLASH_COMMANDS; got:\n{output}"
        );
    }
}
