use crate::session::Message;
use crate::terminal::app::{App, InputMode};

impl App {
    pub(in crate::terminal) fn handle_board(&mut self) -> Option<Message> {
        // #320: handle_board no longer calls `fetch_board_stories()`
        // directly. The fetcher is now async (octocrab), and this
        // handler runs on the sync input-dispatch path. Instead of
        // bridging to the runtime here (which would reintroduce the
        // #294 freeze risk), we signal the event loop's on-demand
        // fetch task via `board_fetch_request_tx`. The result flows
        // back through `board_fetch_result_rx` and is mirrored into
        // `board.stories` + `board_snapshot` by the main select!.
        //
        // Opening the panel first + signaling refresh matches the
        // user-visible behavior of the prior sync path: the panel
        // appears immediately with whatever data is already cached;
        // the new fetch lands within a tick or two.
        self.board.selected = 0;
        self.board.detail = None;
        self.input_mode = InputMode::Project;
        // Unconditionally signal a refresh here (not gated on
        // `board.stories.is_empty()` like `request_board_refresh_if_stale`).
        // `/board` is an explicit user action — they want the latest,
        // even if stale data is already on screen.
        if let Some(tx) = &self.board_fetch_request_tx {
            let _ = tx.try_send(());
        }
        // Complement: also kick the background octocrab poller so
        // `board_snapshot` refreshes on the same beat. No-op when the
        // poller isn't running (headless mode / env unset).
        if let Some(tx) = &self.board_poller_tx {
            let _ = tx.try_send(crate::terminal::board_poller::PollerCommand::RefreshNow);
        }
        self.clear_input();
        None
    }
}
