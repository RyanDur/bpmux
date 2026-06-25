use super::*;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::action::validate_and_log;
use crate::store::ci::CiReduceOutcome;

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

    async fn dispatch_ci(self: &Arc<Self>, action: CiAction) {
        match action {
            CiAction::StartPolling => self.middleware_start_ci().await,
            CiAction::StopPolling => self.middleware_stop(PollKind::Ci).await,
            CiAction::Updated(_) => self.middleware_ci_updated(action).await,
        }
    }

    async fn dispatch_sync(self: &Arc<Self>, action: SyncAction) {
        match action {
            SyncAction::StartIngestPolling => self.middleware_start_ingest().await,
            SyncAction::StopIngestPolling => self.middleware_stop(PollKind::Ingest).await,
            SyncAction::StartHydrationPolling => self.middleware_start_hydration().await,
            SyncAction::StopHydrationPolling => self.middleware_stop(PollKind::Hydration).await,
            SyncAction::StartUpdateGithub { mutation_id } => {
                self.middleware_start_update_github(mutation_id).await
            }
            SyncAction::StopUpdateGithub { mutation_id } => {
                self.middleware_stop_update_github(mutation_id).await
            }
            SyncAction::MutationConfirmed { mutation_id } => {
                self.middleware_mutation_confirmed(mutation_id).await
            }
            SyncAction::MutationRolledBack {
                mutation_id,
                server_story,
            } => {
                self.middleware_mutation_rolled_back(mutation_id, server_story.map(|b| *b))
                    .await;
            }
        }
    }

    pub async fn is_running(&self, kind: PollKind) -> bool {
        let tokens = self.tokens.lock().await;
        tokens.contains_key(&kind)
    }

    async fn spawn_lifecycle_proxy(&self, local_ct: CancellationToken, task_ct: CancellationToken) {
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

    async fn middleware_start_ci(self: &Arc<Self>) {
        let Some(tracker) = self.tracker.as_ref().cloned() else {
            return;
        };
        let mut tokens = self.tokens.lock().await;
        if tokens.contains_key(&PollKind::Ci) {
            return;
        }
        let local_ct = CancellationToken::new();
        let task_ct = CancellationToken::new();
        self.spawn_lifecycle_proxy(local_ct.clone(), task_ct.clone())
            .await;
        crate::transport::ci_watcher::spawn(
            Arc::clone(self),
            tracker,
            self.ci_state_store.clone(),
            task_ct,
        );
        tokens.insert(PollKind::Ci, local_ct);
    }

    async fn middleware_start_ingest(self: &Arc<Self>) {
        let Some(refresh) = self.refresh.as_ref().cloned() else {
            return;
        };
        let mut tokens = self.tokens.lock().await;
        if tokens.contains_key(&PollKind::Ingest) {
            return;
        }
        let local_ct = CancellationToken::new();
        let task_ct = CancellationToken::new();
        self.spawn_lifecycle_proxy(local_ct.clone(), task_ct.clone())
            .await;
        let observer = self.rate_limit_observer.clone();
        let interval = self.ingest_interval;
        tokio::spawn(crate::bridge::run_ingest_poll_loop_cancellable(
            refresh,
            interval,
            Some(observer),
            task_ct,
        ));
        tokens.insert(PollKind::Ingest, local_ct);
    }

    async fn middleware_start_hydration(self: &Arc<Self>) {
        let Some(refresh) = self.refresh.as_ref().cloned() else {
            return;
        };
        let mut tokens = self.tokens.lock().await;
        if tokens.contains_key(&PollKind::Hydration) {
            return;
        }
        let local_ct = CancellationToken::new();
        let task_ct = CancellationToken::new();
        self.spawn_lifecycle_proxy(local_ct.clone(), task_ct.clone())
            .await;
        let observer = self.rate_limit_observer.clone();
        let interval = self.hydration_interval;
        tokio::spawn(crate::bridge::run_hydration_loop_cancellable(
            refresh, interval, observer, task_ct,
        ));
        tokens.insert(PollKind::Hydration, local_ct);
    }

    async fn middleware_stop(&self, kind: PollKind) {
        let mut tokens = self.tokens.lock().await;
        if let Some(token) = tokens.remove(&kind) {
            token.cancel();
        }
    }

    async fn middleware_ci_updated(&self, action: CiAction) {
        let outcome = self.ci_store.apply(&action);
        if outcome == CiReduceOutcome::Changed {
            let snap = match action {
                CiAction::Updated(s) => s,
                _ => return,
            };
            let listeners = {
                let l = self.listeners.lock().unwrap_or_else(|e| e.into_inner());
                l.clone()
            };
            for listener in listeners {
                listener.on_changed(&snap);
            }
        }
    }
}
