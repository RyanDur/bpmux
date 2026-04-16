use crate::session::Message;
use crate::terminal::app::App;

impl App {
    pub(in crate::terminal) fn handle_who(&mut self) -> Option<Message> {
        let mut participants = vec![format!("{} (you)", self.identity.name)];
        for (peer, (status, _)) in &self.peers.activity {
            participants.push(format!("{peer} [{status}]"));
        }
        self.system_message(format!("Participants:\n  {}", participants.join("\n  ")));
        self.clear_input();
        None
    }
}
