use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::github::RateLimitObserver;
use crate::kernel::{Listener, ListenerFuture, Reaction};
use crate::store::app::{AppAction, AppState};
use crate::sync::RefreshDriver;

use super::SyncAction;

pub struct SyncPoll {
    refresh: Arc<dyn RefreshDriver>,
    observer: RateLimitObserver,
    ingest_interval: Duration,
    hydration_interval: Duration,
    parent_ct: CancellationToken,
    ingest_token: Mutex<Option<CancellationToken>>,
    hydration_token: Mutex<Option<CancellationToken>>,
}

impl SyncPoll {
    pub fn new(
        refresh: Arc<dyn RefreshDriver>,
        observer: RateLimitObserver,
        ingest_interval: Duration,
        hydration_interval: Duration,
        parent_ct: CancellationToken,
    ) -> Self {
        Self {
            refresh,
            observer,
            ingest_interval,
            hydration_interval,
            parent_ct,
            ingest_token: Mutex::new(None),
            hydration_token: Mutex::new(None),
        }
    }

    pub fn ingest_running(&self) -> bool {
        self.ingest_token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
    }

    pub fn hydration_running(&self) -> bool {
        self.hydration_token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
    }

    fn start_ingest(&self) {
        let mut guard = self
            .ingest_token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_some() {
            return;
        }
        let token = self.parent_ct.child_token();
        tokio::spawn(crate::bridge::run_ingest_poll_loop_cancellable(
            self.refresh.clone(),
            self.ingest_interval,
            Some(self.observer.clone()),
            token.clone(),
        ));
        *guard = Some(token);
    }

    fn start_hydration(&self) {
        let mut guard = self
            .hydration_token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_some() {
            return;
        }
        let token = self.parent_ct.child_token();
        tokio::spawn(crate::bridge::run_hydration_loop_cancellable(
            self.refresh.clone(),
            self.hydration_interval,
            self.observer.clone(),
            token.clone(),
        ));
        *guard = Some(token);
    }

    fn stop(token: &Mutex<Option<CancellationToken>>) {
        if let Some(ct) = token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            ct.cancel();
        }
    }
}

impl Listener<AppState, AppAction> for SyncPoll {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if let AppAction::Sync(action) = &reaction.action {
            match action {
                SyncAction::StartIngestPolling => self.start_ingest(),
                SyncAction::StopIngestPolling => Self::stop(&self.ingest_token),
                SyncAction::StartHydrationPolling => self.start_hydration(),
                SyncAction::StopHydrationPolling => Self::stop(&self.hydration_token),
                _ => {}
            }
        }
        Box::pin(async {})
    }
}
