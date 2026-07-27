use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

use crate::{AppAction, AppState};
use hub::Hub;
use kernel::{DeliveryMode, Dispatch, Listener, ListenerFuture, Reaction};
use rate_limit::RateLimitObserver;

use super::{CiAction, CiWatchDriver};

pub struct CiPoll {
    hub: Arc<Hub>,
    parent_ct: CancellationToken,
    observer: RateLimitObserver,
    watch_driver: Arc<dyn CiWatchDriver>,
    token: Mutex<Option<CancellationToken>>,
}

impl CiPoll {
    pub fn new(
        hub: Arc<Hub>,
        parent_ct: CancellationToken,
        observer: RateLimitObserver,
        watch_driver: Arc<dyn CiWatchDriver>,
    ) -> Self {
        Self {
            hub,
            parent_ct,
            observer,
            watch_driver,
            token: Mutex::new(None),
        }
    }

    fn start(&self, dispatch: Dispatch<AppAction>) {
        let mut guard = self
            .token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_some() {
            return;
        }
        let token = self.parent_ct.child_token();
        self.watch_driver.spawn_watch(
            dispatch.clone(),
            self.hub.clone(),
            self.observer.clone(),
            token.clone(),
        );
        *guard = Some(token);
        drop(guard);
        dispatch(AppAction::Ci(CiAction::PollingStarted));
    }

    fn stop(&self, dispatch: Dispatch<AppAction>) {
        let taken = self
            .token
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(token) = taken {
            token.cancel();
            dispatch(AppAction::Ci(CiAction::PollingStopped));
        }
    }
}

impl Listener<AppState, AppAction> for CiPoll {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        match reaction.action {
            AppAction::Ci(CiAction::StartPolling) => self.start(reaction.dispatch),
            AppAction::Ci(CiAction::StopPolling) => self.stop(reaction.dispatch),
            _ => {}
        }
        Box::pin(async {})
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::FireAndForget
    }
}
