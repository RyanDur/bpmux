use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::action::validate_and_log;
use crate::store::{PollKind, Store, StoreAction};

impl Store {
    pub async fn dispatch(self: &Arc<Self>, action: StoreAction) {
        validate_and_log(&action);
        match action {
            StoreAction::Ci(a) => self.dispatch_ci(a).await,
            StoreAction::Sync(a) => self.dispatch_sync(a).await,
            StoreAction::Reactions(a) => self.middleware_reactions(a),
            StoreAction::Roster(a) => self.middleware_roster(a),
        }
    }

    pub async fn is_running(&self, kind: PollKind) -> bool {
        let tokens = self.tokens.lock().await;
        tokens.contains_key(&kind)
    }

    pub(crate) async fn spawn_lifecycle_proxy(
        &self,
        local_ct: CancellationToken,
        task_ct: CancellationToken,
    ) {
        let parent_clone = self.parent_ct.clone();
        let local_clone = local_ct.clone();
        let task_clone = task_ct.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = parent_clone.cancelled() => {}
                _ = local_clone.cancelled() => {}
            }
            task_clone.cancel();
        });
    }

    pub(crate) async fn middleware_stop(&self, kind: PollKind) {
        let mut tokens = self.tokens.lock().await;
        if let Some(token) = tokens.remove(&kind) {
            token.cancel();
        }
    }
}
