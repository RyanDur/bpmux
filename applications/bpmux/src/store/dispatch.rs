use std::sync::Arc;

use super::app::{AppAction, validate_and_log};
use crate::store::{PollKind, Store};

#[derive(Clone)]
pub struct Dispatch {
    tx: tokio::sync::mpsc::UnboundedSender<AppAction>,
}

impl Dispatch {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<AppAction>) -> Self {
        Self { tx }
    }

    pub fn dispatch(&self, action: AppAction) {
        let _ = self.tx.send(action);
    }
}

impl Store {
    pub async fn dispatch(self: &Arc<Self>, action: AppAction) {
        validate_and_log(&action);
        self.app_store.dispatch(action);
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
