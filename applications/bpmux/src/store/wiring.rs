use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::{CiPollFetch, Store, app, ci, sync};
use crate::ci::CiStateStore;
use crate::hub;
use crate::kernel;
use crate::sync::RefreshDriver;
use crate::transport::rest::broadcast_listeners::{
    EpicBroadcast, IterationBroadcast, OutcomeBroadcast, StoryBroadcast,
};
use app::OwnerParkListener;

impl Store {
    pub fn new(
        hub: Arc<hub::Hub>,
        tracker: Option<Arc<dyn CiPollFetch>>,
        ci_state_store: CiStateStore,
        parent_ct: CancellationToken,
    ) -> Self {
        Self::with_initial_outbox(hub, tracker, ci_state_store, parent_ct, Vec::new())
    }

    pub fn with_initial_outbox(
        hub: Arc<hub::Hub>,
        tracker: Option<Arc<dyn CiPollFetch>>,
        ci_state_store: CiStateStore,
        parent_ct: CancellationToken,
        initial_outbox: Vec<crate::sync::QueuedWrite>,
    ) -> Self {
        let rate_limit_observer = tracker
            .as_ref()
            .map(|t| t.rate_limit_observer())
            .unwrap_or_default();
        let roster_listeners: app::RosterListeners = Arc::new(Mutex::new(Vec::new()));
        let reactions_listeners: app::ReactionsListeners = Arc::new(Mutex::new(Vec::new()));
        let ci_listeners: app::CiListeners = Arc::new(Mutex::new(Vec::new()));
        let app_store = kernel::Store::new(
            app::AppState::with_board(crate::board::persistence::load(&hub.dir))
                .with_shared_history(crate::product_management::history_persistence::load(
                    &hub.dir,
                ))
                .with_local_history(crate::board::history_persistence::load(&hub.dir))
                .with_outbox(initial_outbox),
            Box::new(app::app_reduce),
        );
        app_store.add_listener(Arc::new(app::RosterBroadcast::new(
            roster_listeners.clone(),
        )));
        app_store.add_listener(Arc::new(app::ReactionsBroadcast::new(
            reactions_listeners.clone(),
        )));
        app_store.add_listener(Arc::new(app::CiBroadcast::new(ci_listeners.clone())));
        let ci_poll = Arc::new(ci::CiPoll::new(
            tracker.clone(),
            ci_state_store.clone(),
            hub.clone(),
            parent_ct.clone(),
            rate_limit_observer.clone(),
        ));
        app_store.add_listener(ci_poll.clone());
        Self {
            hub,
            parent_ct,
            rate_limit_observer,
            ci_poll,
            sync_poll: None,
            github_drain: None,
            app_store,
            ci_listeners,
            roster_listeners,
            reactions_listeners,
        }
    }

    pub fn with_refresh(
        mut self,
        refresh: Arc<dyn RefreshDriver>,
        ingest_interval: Duration,
        hydration_interval: Duration,
    ) -> Self {
        let sync_poll = Arc::new(sync::SyncPoll::new(
            refresh.clone(),
            self.rate_limit_observer.clone(),
            ingest_interval,
            hydration_interval,
            self.parent_ct.clone(),
        ));
        self.app_store.add_listener(sync_poll.clone());
        self.app_store.add_listener(Arc::new(OutcomeBroadcast::new(
            self.hub.clone(),
            refresh.clone(),
        )));
        self.app_store
            .add_listener(Arc::new(IterationBroadcast::new(
                self.hub.clone(),
                refresh.clone(),
            )));
        self.app_store.add_listener(Arc::new(EpicBroadcast::new(
            self.hub.clone(),
            refresh.clone(),
        )));
        self.app_store.add_listener(Arc::new(StoryBroadcast::new(
            self.hub.clone(),
            refresh.clone(),
        )));
        self.app_store.add_listener(Arc::new(OwnerParkListener));
        self.app_store
            .add_listener(Arc::new(app::BoardPlacementReconciler));
        self.app_store
            .add_listener(Arc::new(app::BoardPlacementPersister::new(
                self.hub.clone(),
            )));
        self.app_store
            .add_listener(Arc::new(app::SharedHistoryPersister::new(self.hub.clone())));
        self.app_store
            .add_listener(Arc::new(app::LocalHistoryPersister::new(self.hub.clone())));
        self.app_store
            .add_listener(Arc::new(sync::OutboxPersister::new(
                sync::OutboxSink::Storage(self.hub.storage.clone()),
            )));
        let github_drain = Arc::new(sync::GithubDrain::new(refresh, self.parent_ct.clone()));
        self.app_store.add_listener(github_drain.clone());
        self.sync_poll = Some(sync_poll);
        self.github_drain = Some(github_drain);
        self
    }
}
