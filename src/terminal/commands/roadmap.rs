use crate::session::Message;
use crate::terminal::app::App;
use crate::terminal::board;

impl App {
    pub(crate) fn handle_roadmap(&mut self) -> Option<Message> {
        let roadmap_text = match board::fetch_roadmap() {
            Ok(text) => text,
            Err(_) => "Roadmap unavailable.".to_string(),
        };
        self.system_message(roadmap_text);
        self.clear_input();
        None
    }
}
