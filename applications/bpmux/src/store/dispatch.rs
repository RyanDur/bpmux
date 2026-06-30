use std::sync::Arc;

use super::action::validate_and_log;
use crate::store::{PollKind, Store, StoreAction};

impl Store {
    pub async fn dispatch(self: &Arc<Self>, action: StoreAction) {
        validate_and_log(&action);
        match action {
            StoreAction::Ci(a) => self.app_store.dispatch(a),
            StoreAction::Sync(a) => self.app_store.dispatch(a),
            StoreAction::Reactions(a) => self.app_store.dispatch(a),
            StoreAction::Roster(a) => self.app_store.dispatch(a),
            StoreAction::Outcome(a) => self.app_store.dispatch(a),
        }
    }

    pub async fn is_running(&self, kind: PollKind) -> bool {
        match kind {
            PollKind::Ci => self.ci_poll.is_running(),
            PollKind::Ingest => self
                .sync_poll
                .as_ref()
                .is_some_and(|poll| poll.ingest_running()),
            PollKind::Hydration => self
                .sync_poll
                .as_ref()
                .is_some_and(|poll| poll.hydration_running()),
        }
    }
}
