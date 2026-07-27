use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use kernel::{DeliveryMode, Dispatch, Listener, ListenerFuture, Reaction};
use rate_limit::RateLimitObserver;
use store::{AppAction, AppState, SyncAction, SyncHealthAction};
use sync::SyncOrchestrator;

pub struct SyncPoll {
    refresh: Arc<SyncOrchestrator>,
    select: store::Select,
    observer: RateLimitObserver,
    connected: super::ConnectedProbe,
    ingest_interval: Duration,
    hydration_interval: Duration,
    parent_ct: CancellationToken,
    ingest_token: Mutex<Option<CancellationToken>>,
    hydration_token: Mutex<Option<CancellationToken>>,
}

impl SyncPoll {
    pub fn new(
        refresh: Arc<SyncOrchestrator>,
        select: store::Select,
        observer: RateLimitObserver,
        ingest_interval: Duration,
        hydration_interval: Duration,
        parent_ct: CancellationToken,
        connected: super::ConnectedProbe,
    ) -> Self {
        Self {
            refresh,
            select,
            observer,
            connected,
            ingest_interval,
            hydration_interval,
            parent_ct,
            ingest_token: Mutex::new(None),
            hydration_token: Mutex::new(None),
        }
    }

    fn start_ingest(&self, dispatch: Dispatch<AppAction>) {
        let mut guard = self
            .ingest_token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_some() {
            return;
        }
        let token = self.parent_ct.child_token();
        tokio::spawn(crate::run_ingest_poll_loop_cancellable(
            self.refresh.clone(),
            self.ingest_interval,
            Some(self.observer.clone()),
            token.clone(),
            self.connected.clone(),
        ));
        *guard = Some(token);
        drop(guard);
        dispatch(AppAction::SyncHealth(
            SyncHealthAction::IngestPollingStarted,
        ));
    }

    fn start_hydration(&self, dispatch: Dispatch<AppAction>) {
        let mut guard = self
            .hydration_token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_some() {
            return;
        }
        let token = self.parent_ct.child_token();
        tokio::spawn(crate::run_hydration_loop_cancellable(
            self.refresh.clone(),
            self.select.clone(),
            self.hydration_interval,
            self.observer.clone(),
            token.clone(),
            self.connected.clone(),
        ));
        *guard = Some(token);
        drop(guard);
        dispatch(AppAction::SyncHealth(
            SyncHealthAction::HydrationPollingStarted,
        ));
    }

    fn stop_ingest(&self, dispatch: Dispatch<AppAction>) {
        let taken = self
            .ingest_token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(ct) = taken {
            ct.cancel();
            dispatch(AppAction::SyncHealth(
                SyncHealthAction::IngestPollingStopped,
            ));
        }
    }

    fn stop_hydration(&self, dispatch: Dispatch<AppAction>) {
        let taken = self
            .hydration_token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(ct) = taken {
            ct.cancel();
            dispatch(AppAction::SyncHealth(
                SyncHealthAction::HydrationPollingStopped,
            ));
        }
    }
}

impl Listener<AppState, AppAction> for SyncPoll {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if let AppAction::Sync(action) = &reaction.action {
            match action {
                SyncAction::StartIngestPolling => self.start_ingest(reaction.dispatch),
                SyncAction::StopIngestPolling => self.stop_ingest(reaction.dispatch),
                SyncAction::StartHydrationPolling => self.start_hydration(reaction.dispatch),
                SyncAction::StopHydrationPolling => self.stop_hydration(reaction.dispatch),
                _ => {}
            }
        }
        Box::pin(async {})
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::FireAndForget
    }
}
