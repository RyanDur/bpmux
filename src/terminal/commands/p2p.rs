use crate::session::Message;
use crate::terminal::app::App;
use crate::transport::peer::P2pCommand;

impl App {
    pub(in crate::terminal) fn handle_session(&mut self) -> Option<Message> {
        self.system_message("Creating P2P session... (gathering ICE candidates)".to_string());
        self.p2p_command = Some(P2pCommand::CreateSession);
        self.clear_input();
        None
    }

    pub(in crate::terminal) fn handle_join(&mut self, args: String) -> Option<Message> {
        let args = args.trim();
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if args.is_empty() || parts.len() < 2 {
            self.system_message("/join <passphrase> <offer-code>".to_string());
            self.clear_input();
            return None;
        }
        let passphrase = parts[0].to_string();
        let offer_code = parts[1].trim().to_string();
        self.system_message(
            "Joining session... (decrypting offer, gathering ICE candidates)".to_string(),
        );
        self.p2p_command = Some(P2pCommand::JoinSession {
            passphrase,
            offer_code,
        });
        self.clear_input();
        None
    }

    pub(in crate::terminal) fn handle_accept(&mut self, answer_code: String) -> Option<Message> {
        let answer_code = answer_code.trim().to_string();
        if answer_code.is_empty() {
            self.system_message("/accept <answer-code>".to_string());
            self.clear_input();
            return None;
        }
        self.system_message("Accepting answer... (establishing connection)".to_string());
        self.p2p_command = Some(P2pCommand::AcceptAnswer { answer_code });
        self.clear_input();
        None
    }
}
