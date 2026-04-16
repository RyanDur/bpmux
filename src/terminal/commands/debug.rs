use crate::session::Message;
use crate::terminal::app::App;

impl App {
    pub(in crate::terminal) fn handle_debug(&mut self) -> Option<Message> {
        self.display.show_system_messages = !self.display.show_system_messages;
        let state = if self.display.show_system_messages {
            "on"
        } else {
            "off"
        };
        self.system_message(format!(
            "Debug mode {state}. System messages are now {}.",
            if self.display.show_system_messages {
                "visible"
            } else {
                "hidden"
            }
        ));
        self.clear_input();
        None
    }
}
