#[derive(Clone, Debug)]
pub enum RoutingAction {
    Registered {
        user_id: String,
        display: String,
        role: Option<String>,
    },
    TerminalLinked {
        user_id: String,
        terminal_id: String,
    },
    Renamed {
        user_id: String,
        display: String,
    },
    CursorAdvanced {
        user_id: String,
        cursor: String,
    },
    CursorAnchored {
        user_id: String,
        cursor: String,
    },
    Reaped {
        display: String,
    },
}
