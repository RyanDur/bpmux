use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::kernel::{DeliveryMode, Listener, ListenerFuture, Reaction};
use crate::store::app::{AppAction, AppState};
use crate::sync::RefreshDriver;

pub struct GithubDrain {
    refresh: Arc<dyn RefreshDriver>,
    parent_ct: CancellationToken,
}

impl GithubDrain {
    pub fn new(refresh: Arc<dyn RefreshDriver>, parent_ct: CancellationToken) -> Self {
        Self { refresh, parent_ct }
    }
}

impl Listener<AppState, AppAction> for GithubDrain {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.sync, &reaction.current.sync) {
            return Box::pin(async {});
        }
        if reaction.current.sync.queue.is_empty() {
            return Box::pin(async {});
        }
        let refresh = self.refresh.clone();
        let token = self.parent_ct.clone();
        tokio::spawn(async move {
            let outcome = tokio::select! {
                _ = token.cancelled() => return,
                drained = refresh.drain_pending() => drained,
            };
            match outcome {
                Ok(0) => {}
                Ok(n) => {
                    crate::diag::log_diag(&format!("github drain: pushed {n} event(s)"));
                }
                Err(e) => {
                    crate::diag::log_diag(&format!("github drain: {e}"));
                }
            }
        });
        Box::pin(async {})
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::FireAndForget
    }
}
