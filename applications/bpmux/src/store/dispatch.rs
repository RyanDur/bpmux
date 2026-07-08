use std::sync::Arc;

use super::app::{AppAction, validate_and_log};
use crate::store::{PollKind, Store};

#[derive(Clone)]
pub struct Dispatch {
    dispatch: crate::kernel::Dispatch<AppAction>,
}

impl Dispatch {
    pub fn new(dispatch: crate::kernel::Dispatch<AppAction>) -> Self {
        Self { dispatch }
    }

    pub fn dispatch(&self, action: AppAction) {
        (self.dispatch)(action);
    }

    #[cfg(test)]
    pub(crate) fn recording() -> (Self, tokio::sync::mpsc::UnboundedReceiver<AppAction>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AppAction>();
        (
            Self::new(Arc::new(move |action| {
                let _ = tx.send(action);
            })),
            rx,
        )
    }
}

impl Store {
    pub fn dispatcher(&self) -> Dispatch {
        let weak = Arc::downgrade(&self.app_store);
        Dispatch::new(Arc::new(move |action: AppAction| {
            if let Some(store) = weak.upgrade() {
                validate_and_log(&action);
                store.dispatch(action);
            }
        }))
    }

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
