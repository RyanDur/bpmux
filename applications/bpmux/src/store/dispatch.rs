use std::sync::Arc;

use super::app::{AppAction, AppState, validate_and_log};
use crate::store::{PollKind, Store};

#[derive(Clone)]
pub struct Dispatch {
    dispatch: kernel::Dispatch<AppAction>,
}

impl Dispatch {
    pub fn new(dispatch: kernel::Dispatch<AppAction>) -> Self {
        Self { dispatch }
    }

    pub(crate) fn for_app_store(app_store: &Arc<kernel::Store<AppState, AppAction>>) -> Self {
        let weak = Arc::downgrade(app_store);
        Self::new(Arc::new(move |action: AppAction| {
            if let Some(store) = weak.upgrade() {
                validate_and_log(&action);
                store.dispatch(action);
            }
        }))
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
        Dispatch::for_app_store(&self.app_store)
    }

    pub async fn dispatch(self: &Arc<Self>, action: AppAction) {
        validate_and_log(&action);
        self.app_store.dispatch(action);
    }

    pub async fn is_running(&self, kind: PollKind) -> bool {
        match kind {
            PollKind::Ci => self.app_store.select(|state| super::ci::polling(&state.ci)),
            PollKind::Ingest => self
                .app_store
                .select(|state| super::sync_health::ingest_polling(&state.sync_health)),
            PollKind::Hydration => self
                .app_store
                .select(|state| super::sync_health::hydration_polling(&state.sync_health)),
        }
    }
}
