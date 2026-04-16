use crate::session::Message;
use crate::terminal::app::App;

impl App {
    pub(in crate::terminal) fn handle_quit(&mut self) -> Option<Message> {
        self.lifecycle.should_quit = true;
        None
    }
}
