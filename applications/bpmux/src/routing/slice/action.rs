#[derive(Clone, Debug)]
pub enum RoutingAction {
    Registered {
        session_id: String,
        display: String,
    },
    TerminalLinked {
        session_id: String,
        terminal_id: String,
    },
    Renamed {
        session_id: String,
        display: String,
    },
    CursorAdvanced {
        session_id: String,
        cursor: String,
    },
    CursorAnchored {
        session_id: String,
        cursor: String,
    },
    Reaped {
        display: String,
    },
}
