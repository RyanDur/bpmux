use crate::session::Message;
use crate::terminal::app::{App, InputMode};

impl App {
    /// Agent-triggered view-open for the backlog (#286). Flips the
    /// input mode to Project — the project panel already renders the
    /// board section (#227). Idempotent: if the panel is already open,
    /// the mode assignment is a no-op. Ryan's in-progress input is
    /// preserved (#210 drive-by fix pattern) — the agent emitted this
    /// in response to a prose-mention of the backlog, so wiping
    /// whatever Ryan happens to be typing would be a surprise.
    pub(in crate::terminal) fn handle_view_backlog(&mut self) -> Option<Message> {
        self.input_mode = InputMode::Project;
        self.request_board_refresh_if_stale();
        None
    }
}
