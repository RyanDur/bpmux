use super::app::App;
use tui_textarea::CursorMove;

/// Registered slash commands with brief one-line usage descriptions.
///
/// **Single source of truth (#164)** — used by:
/// - the textarea popup completion (this module)
/// - the `?` keybind help overlay (`ui.rs::render_help_overlay`)
/// - the `/help` slash command (`commands.rs::handle_help`)
///
/// Adding a new command means adding ONE line here. The popup, overlay,
/// and `/help` all pick it up automatically — no parallel lists to keep
/// in sync.
///
/// **Tuple shape**: `(name, short_desc, long_desc)`. The short form fits
/// in the compact 2-column help overlay (≤14 chars per entry). The long
/// form carries full detail for the `/help` chat-pane output where width
/// is generous. Two versions of the same information — each at the right
/// granularity for its surface. See `commands_help::format_command_list`
/// for the consumer logic.
///
/// **Why two descriptions** (#164 re-delivery): the original #164 shipped
/// with a single description, and the 2-column overlay branch dropped it
/// entirely for width reasons — producing an overlay that showed command
/// names with no hint at what each command did. Ryan correctly flagged
/// that as a regression. The fix carries short descriptions for the
/// compact surface and long descriptions for the chat-pane surface.
///
/// Convention for short: single verb or verb-phrase, sentence case, no
/// period, ≤14 chars. Convention for long: full sentence fragment, no
/// period, fits on one line.
pub(super) const SLASH_COMMANDS: &[(&str, &str, &str)] = &[
    (
        "/to",
        "send to roles",
        "send to recipients (comma-separated)",
    ),
    (
        "/share",
        "widen scope",
        "propagate a message to a wider scope",
    ),
    ("/view", "switch view", "list channels or switch view"),
    (
        "/all",
        "toggle filter",
        "toggle classifier-driven view filter",
    ),
    ("/search", "find in log", "search the conversation log"),
    (
        "/lessons",
        "team lessons",
        "show team lessons (or peer's with arg)",
    ),
    ("/board", "project board", "show project board"),
    ("/roadmap", "roadmap doc", "show project roadmap"),
    ("/status", "set status", "declare your status"),
    ("/who", "list peers", "list participants"),
    ("/session", "start p2p", "create a P2P session"),
    ("/join", "join p2p", "join a remote P2P session"),
    ("/accept", "finish p2p", "complete a P2P handshake"),
    (
        "/debug",
        "sys msg toggle",
        "toggle system message visibility",
    ),
    ("/tokens", "cost view", "show token cost by role"),
    (
        "/focus",
        "role filter",
        "filter chat view by role (e.g. /focus dev,pm)",
    ),
    ("/help", "show this", "show this list"),
    ("/quit", "exit", "exit bpmux"),
];

impl App {
    pub fn insert_char(&mut self, c: char) {
        self.textarea.insert_char(c);
        self.sync_input_from_textarea();
        self.popup_selected = None;
    }

    pub fn delete_char(&mut self) {
        self.textarea.delete_char();
        self.sync_input_from_textarea();
        self.popup_selected = None;
    }

    pub fn move_cursor_left(&mut self) {
        self.textarea.move_cursor(CursorMove::Back);
        self.sync_input_from_textarea();
    }

    pub fn move_cursor_right(&mut self) {
        self.textarea.move_cursor(CursorMove::Forward);
        self.sync_input_from_textarea();
    }

    pub fn move_cursor_home(&mut self) {
        self.textarea.move_cursor(CursorMove::Head);
        self.sync_input_from_textarea();
    }

    pub fn move_cursor_end(&mut self) {
        self.textarea.move_cursor(CursorMove::End);
        self.sync_input_from_textarea();
    }

    pub fn kill_to_end(&mut self) {
        self.textarea.delete_line_by_end();
        self.sync_input_from_textarea();
    }

    pub fn delete_word_backward(&mut self) {
        self.textarea.delete_word();
        self.sync_input_from_textarea();
    }

    pub fn move_word_left(&mut self) {
        self.textarea.move_cursor(CursorMove::WordBack);
        self.sync_input_from_textarea();
    }

    pub fn move_word_right(&mut self) {
        self.textarea.move_cursor(CursorMove::WordForward);
        self.sync_input_from_textarea();
    }

    pub fn clear_input(&mut self) {
        let mut ta = tui_textarea::TextArea::default();
        ta.set_hard_tab_indent(false);
        ta.set_wrap_mode(tui_textarea::WrapMode::Word);
        self.textarea = ta;
        self.sync_input_from_textarea();
    }

    pub fn history_up(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let new_index = match self.history_index {
            None => self.input_history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.history_index = Some(new_index);
        self.set_textarea_content(&self.input_history[new_index].clone());
    }

    pub fn history_down(&mut self) {
        let Some(current) = self.history_index else {
            return;
        };
        if current + 1 >= self.input_history.len() {
            self.history_index = None;
            self.clear_input();
        } else {
            let new_index = current + 1;
            self.history_index = Some(new_index);
            self.set_textarea_content(&self.input_history[new_index].clone());
        }
    }

    pub fn tab_complete(&mut self) -> bool {
        let input = self.input.trim_start();
        let rest = match input.strip_prefix("/to ") {
            Some(r) => r,
            None => return false,
        };
        if rest.contains(' ') {
            return false;
        }
        let (prefix_part, partial) = match rest.rfind(',') {
            Some(i) => (&rest[..=i], rest[i + 1..].trim()),
            None => ("", rest.trim()),
        };
        let mut candidates: Vec<String> = self
            .peer_activity
            .keys()
            .map(|k| k.split(':').next().unwrap_or(k).to_string())
            .filter(|name| name.starts_with(partial) && name != partial)
            .collect();
        candidates.sort();
        if candidates.is_empty() {
            return false;
        }
        let completed = &candidates[0];
        let new_input = format!("/to {}{} ", prefix_part, completed);
        self.set_textarea_content(&new_input);
        true
    }

    /// Popup completion matches for the typed prefix. Returns
    /// `(name, short_desc)` pairs — the popup is a compact typing-time
    /// surface so the short form is appropriate. The long form lives on
    /// the `/help` chat-pane surface only.
    pub fn popup_matches(&self) -> Vec<(&'static str, &'static str)> {
        if !self.input.starts_with('/') || self.input.contains(' ') {
            return vec![];
        }
        let typed = self.input.split_whitespace().next().unwrap_or(&self.input);
        SLASH_COMMANDS
            .iter()
            .filter(|(cmd, _, _)| cmd.starts_with(typed))
            .map(|(cmd, short, _)| (*cmd, *short))
            .collect()
    }

    pub fn popup_visible(&self) -> bool {
        !self.popup_matches().is_empty()
    }

    pub fn popup_up(&mut self) {
        let count = self.popup_matches().len();
        if count == 0 {
            return;
        }
        self.popup_selected = Some(match self.popup_selected {
            None | Some(0) => count - 1,
            Some(i) => i - 1,
        });
    }

    pub fn popup_down(&mut self) {
        let count = self.popup_matches().len();
        if count == 0 {
            return;
        }
        self.popup_selected = Some(match self.popup_selected {
            None => 0,
            Some(i) if i + 1 >= count => 0,
            Some(i) => i + 1,
        });
    }

    pub fn popup_accept(&mut self) -> bool {
        let matches = self.popup_matches();
        if let Some(idx) = self.popup_selected
            && let Some((cmd, _)) = matches.get(idx)
        {
            let new_input = format!("{} ", cmd);
            self.set_textarea_content(&new_input);
            self.popup_selected = None;
            return true;
        }
        false
    }

    pub fn popup_dismiss(&mut self) {
        self.popup_selected = None;
    }
}
