use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

use crate::ci::CiStateStore;
use crate::github::RateLimitObserver;
use crate::session::Session;
use crate::store::kernel::{Dispatch, Listener, ListenerFuture, Reaction};
use crate::store::{AppAction, AppState};
use crate::transport::ci_watcher;

use super::{CiAction, CiPollFetch};

pub struct CiPoll {
    tracker: Option<Arc<dyn CiPollFetch>>,
    ci_state_store: CiStateStore,
    session: Arc<Session>,
    parent_ct: CancellationToken,
    observer: RateLimitObserver,
    token: Mutex<Option<CancellationToken>>,
}

impl CiPoll {
    pub fn new(
        tracker: Option<Arc<dyn CiPollFetch>>,
        ci_state_store: CiStateStore,
        session: Arc<Session>,
        parent_ct: CancellationToken,
        observer: RateLimitObserver,
    ) -> Self {
        Self {
            tracker,
            ci_state_store,
            session,
            parent_ct,
            observer,
            token: Mutex::new(None),
        }
    }

    pub fn is_running(&self) -> bool {
        self.token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
    }

    fn start(&self, dispatch: Dispatch<AppAction>) {
        let Some(tracker) = self.tracker.clone() else {
            return;
        };
        let mut guard = self
            .token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_some() {
            return;
        }
        let token = self.parent_ct.child_token();
        ci_watcher::spawn(
            dispatch,
            tracker,
            self.ci_state_store.clone(),
            self.session.clone(),
            self.observer.clone(),
            token.clone(),
        );
        *guard = Some(token);
    }

    fn stop(&self) {
        if let Some(token) = self
            .token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            token.cancel();
        }
    }
}

impl Listener<AppState, AppAction> for CiPoll {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        match reaction.action {
            AppAction::Ci(CiAction::StartPolling) => self.start(reaction.dispatch),
            AppAction::Ci(CiAction::StopPolling) => self.stop(),
            _ => {}
        }
        Box::pin(async {})
    }
}
