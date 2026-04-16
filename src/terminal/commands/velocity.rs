use crate::session::Message;
use crate::terminal::app::{App, InputMode};

impl App {
    /// Agent-triggered view-open for velocity (#210). Flips the
    /// input mode to Project — velocity already renders inside that
    /// panel (#227). No input clearing here: the trigger is agent-
    /// driven, so wiping whatever Ryan happens to be typing would
    /// be a surprise, not a feature.
    pub(in crate::terminal) fn handle_velocity(&mut self) -> Option<Message> {
        self.input_mode = InputMode::Project;
        None
    }
}
