use super::app::App;
use crate::session::Message;
use tui_textarea::CursorMove;

/// A slash-command registration. One source of truth for:
/// - the textarea popup completion (this module)
/// - the `?` keybind help overlay (`ui::render_help_overlay`)
/// - the `/help` slash command (`commands::help::handle_help`)
/// - dispatch from `App::submit_message`
///
/// Adding a new command means adding ONE entry here. The compiler
/// enforces that every registered command has a handler because the
/// function pointer is mandatory — previously, palette entry and
/// dispatch lived in parallel lists that could drift silently.
///
/// Convention for short: single verb or verb-phrase, sentence case,
/// no period, ≤14 chars. Convention for long: full sentence
/// fragment, no period, fits on one line.
#[derive(Clone, Copy)]
pub(super) struct SlashCommand {
    pub name: &'static str,
    pub short_desc: &'static str,
    pub long_desc: &'static str,
    pub handler: fn(&mut App, &str) -> Option<Message>,
}

/// Single source of truth for slash commands. Entry order controls
/// popup/overlay display order.
pub(super) const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "/to",
        short_desc: "send to roles",
        long_desc: "send to recipients (comma-separated)",
        handler: dispatch_to,
    },
    SlashCommand {
        name: "/share",
        short_desc: "widen scope",
        long_desc: "propagate a message to a wider scope",
        handler: dispatch_share,
    },
    SlashCommand {
        name: "/all",
        short_desc: "toggle filter",
        long_desc: "toggle classifier-driven view filter",
        handler: dispatch_all,
    },
    SlashCommand {
        name: "/search",
        short_desc: "find in log",
        long_desc: "search the conversation log",
        handler: dispatch_search,
    },
    SlashCommand {
        name: "/lessons",
        short_desc: "team lessons",
        long_desc: "show team lessons (or peer's with arg)",
        handler: dispatch_lessons,
    },
    SlashCommand {
        name: "/board",
        short_desc: "project board",
        long_desc: "show project board",
        handler: dispatch_board,
    },
    SlashCommand {
        name: "/status",
        short_desc: "set status",
        long_desc: "declare your status",
        handler: dispatch_status,
    },
    SlashCommand {
        name: "/who",
        short_desc: "list peers",
        long_desc: "list participants",
        handler: dispatch_who,
    },
    SlashCommand {
        name: "/session",
        short_desc: "start p2p",
        long_desc: "create a P2P session",
        handler: dispatch_session,
    },
    SlashCommand {
        name: "/join",
        short_desc: "join p2p",
        long_desc: "join a remote P2P session",
        handler: dispatch_join,
    },
    SlashCommand {
        name: "/accept",
        short_desc: "finish p2p",
        long_desc: "complete a P2P handshake",
        handler: dispatch_accept,
    },
    SlashCommand {
        name: "/debug",
        short_desc: "sys msg toggle",
        long_desc: "toggle system message visibility",
        handler: dispatch_debug,
    },
    SlashCommand {
        name: "/focus",
        short_desc: "role filter",
        long_desc: "filter chat view by role (e.g. /focus dev,pm)",
        handler: dispatch_focus,
    },
    SlashCommand {
        name: "/identity",
        short_desc: "agent traits",
        long_desc: "show identity traits for a role (e.g. /identity dev)",
        handler: dispatch_identity,
    },
    SlashCommand {
        name: "/topic",
        short_desc: "switch topic",
        long_desc: "manage topics (e.g. /topic create scroll-fix)",
        handler: dispatch_topic,
    },
    SlashCommand {
        name: "/help",
        short_desc: "show this",
        long_desc: "show this list",
        handler: dispatch_help,
    },
    SlashCommand {
        name: "/quit",
        short_desc: "exit",
        long_desc: "exit bpmux",
        handler: dispatch_quit,
    },
];

pub(super) fn find_slash_command(name: &str) -> Option<&'static SlashCommand> {
    SLASH_COMMANDS.iter().find(|c| c.name == name)
}

// ── Dispatch wrappers ──────────────────────────────────────────────
// Each wrapper adapts a `fn(&mut App, &str)` signature to the
// handler's own shape. The registry holds these so the compiler
// proves every entry has a callable. Handlers themselves keep their
// original signatures — this is purely the adapter layer.

fn dispatch_to(app: &mut App, args: &str) -> Option<Message> {
    app.handle_to(args.to_string())
}
fn dispatch_share(app: &mut App, args: &str) -> Option<Message> {
    app.handle_share(args.to_string())
}
fn dispatch_all(app: &mut App, _args: &str) -> Option<Message> {
    app.handle_toggle_all()
}
fn dispatch_search(app: &mut App, args: &str) -> Option<Message> {
    app.handle_search(args.to_string())
}
fn dispatch_lessons(app: &mut App, args: &str) -> Option<Message> {
    if args.is_empty() {
        app.handle_lessons()
    } else {
        app.handle_peer_lessons(args.trim().to_string())
    }
}
fn dispatch_board(app: &mut App, _args: &str) -> Option<Message> {
    app.handle_board()
}
fn dispatch_status(app: &mut App, args: &str) -> Option<Message> {
    // handle_status expects the full `/status [text]` as the activity
    // message body so peers see the original slash form in their
    // activity surface. Rebuild it from cmd + args.
    let full = if args.is_empty() {
        "/status".to_string()
    } else {
        format!("/status {args}")
    };
    app.handle_status(full)
}
fn dispatch_who(app: &mut App, _args: &str) -> Option<Message> {
    app.handle_who()
}
fn dispatch_session(app: &mut App, _args: &str) -> Option<Message> {
    app.handle_session()
}
fn dispatch_join(app: &mut App, args: &str) -> Option<Message> {
    app.handle_join(args.to_string())
}
fn dispatch_accept(app: &mut App, args: &str) -> Option<Message> {
    app.handle_accept(args.trim().to_string())
}
fn dispatch_debug(app: &mut App, _args: &str) -> Option<Message> {
    app.handle_debug()
}
fn dispatch_focus(app: &mut App, args: &str) -> Option<Message> {
    app.handle_focus(args.to_string())
}
fn dispatch_identity(app: &mut App, args: &str) -> Option<Message> {
    app.handle_identity(args.to_string())
}
fn dispatch_topic(app: &mut App, args: &str) -> Option<Message> {
    app.handle_topic(args.to_string())
}
fn dispatch_help(app: &mut App, _args: &str) -> Option<Message> {
    app.handle_help()
}
fn dispatch_quit(app: &mut App, _args: &str) -> Option<Message> {
    app.handle_quit()
}

impl App {
    pub fn insert_char(&mut self, c: char) {
        self.editor.textarea.insert_char(c);
        self.sync_input_from_textarea();
        self.editor.popup_selected = None;
    }

    pub fn delete_char(&mut self) {
        self.editor.textarea.delete_char();
        self.sync_input_from_textarea();
        self.editor.popup_selected = None;
    }

    pub fn move_cursor_left(&mut self) {
        self.editor.textarea.move_cursor(CursorMove::Back);
        self.sync_input_from_textarea();
    }

    pub fn move_cursor_right(&mut self) {
        self.editor.textarea.move_cursor(CursorMove::Forward);
        self.sync_input_from_textarea();
    }

    pub fn move_cursor_home(&mut self) {
        self.editor.textarea.move_cursor(CursorMove::Head);
        self.sync_input_from_textarea();
    }

    pub fn move_cursor_end(&mut self) {
        self.editor.textarea.move_cursor(CursorMove::End);
        self.sync_input_from_textarea();
    }

    pub fn kill_to_end(&mut self) {
        self.editor.textarea.delete_line_by_end();
        self.sync_input_from_textarea();
    }

    pub fn delete_word_backward(&mut self) {
        self.editor.textarea.delete_word();
        self.sync_input_from_textarea();
    }

    pub fn move_word_left(&mut self) {
        self.editor.textarea.move_cursor(CursorMove::WordBack);
        self.sync_input_from_textarea();
    }

    pub fn move_word_right(&mut self) {
        self.editor.textarea.move_cursor(CursorMove::WordForward);
        self.sync_input_from_textarea();
    }

    pub fn clear_input(&mut self) {
        let mut ta = tui_textarea::TextArea::default();
        ta.set_hard_tab_indent(false);
        ta.set_wrap_mode(tui_textarea::WrapMode::Word);
        self.editor.textarea = ta;
        self.sync_input_from_textarea();
    }

    pub fn history_up(&mut self) {
        if self.editor.input_history.is_empty() {
            return;
        }
        let new_index = match self.editor.history_index {
            None => self.editor.input_history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.editor.history_index = Some(new_index);
        self.set_textarea_content(&self.editor.input_history[new_index].clone());
    }

    pub fn history_down(&mut self) {
        let Some(current) = self.editor.history_index else {
            return;
        };
        if current + 1 >= self.editor.input_history.len() {
            self.editor.history_index = None;
            self.clear_input();
        } else {
            let new_index = current + 1;
            self.editor.history_index = Some(new_index);
            self.set_textarea_content(&self.editor.input_history[new_index].clone());
        }
    }

    pub fn tab_complete(&mut self) -> bool {
        let input = self.editor.input.trim_start();
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
            .peers
            .activity
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
        if !self.editor.input.starts_with('/') || self.editor.input.contains(' ') {
            return vec![];
        }
        let typed = self.editor.input.split_whitespace().next().unwrap_or(&self.editor.input);
        SLASH_COMMANDS
            .iter()
            .filter(|c| c.name.starts_with(typed))
            .map(|c| (c.name, c.short_desc))
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
        self.editor.popup_selected = Some(match self.editor.popup_selected {
            None | Some(0) => count - 1,
            Some(i) => i - 1,
        });
    }

    pub fn popup_down(&mut self) {
        let count = self.popup_matches().len();
        if count == 0 {
            return;
        }
        self.editor.popup_selected = Some(match self.editor.popup_selected {
            None => 0,
            Some(i) if i + 1 >= count => 0,
            Some(i) => i + 1,
        });
    }

    pub fn popup_accept(&mut self) -> bool {
        let matches = self.popup_matches();
        if let Some(idx) = self.editor.popup_selected
            && let Some((cmd, _)) = matches.get(idx)
        {
            let new_input = format!("{} ", cmd);
            self.set_textarea_content(&new_input);
            self.editor.popup_selected = None;
            return true;
        }
        false
    }

    pub fn popup_dismiss(&mut self) {
        self.editor.popup_selected = None;
    }
}
