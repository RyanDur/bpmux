use crate::session::Message;
use crate::terminal::app::{App, InputMode};
use crate::terminal::board;

impl App {
    pub(in crate::terminal) fn handle_board(&mut self) -> Option<Message> {
        match board::fetch_board_stories() {
            Ok(stories) => {
                self.board.stories = stories;
                self.board.selected = 0;
                self.board.detail = None;
                // Open in project panel (split view) by default.
                // The full-screen board is still accessible via the
                // board_detail navigation.
                self.input_mode = InputMode::Project;
            }
            Err(_) => {
                self.system_message(
                    "Board unavailable. Check gh CLI is installed and authenticated.".to_string(),
                );
            }
        }
        self.clear_input();
        None
    }
}
