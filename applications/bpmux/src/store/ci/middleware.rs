use super::*;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::store::stores::{
    ReactionsAction, RosterAction, StoreReduceOutcome, apply_reactions, apply_roster,
};

impl Store {
    pub async fn dispatch(self: &Arc<Self>, action: StoreAction) {
        validate_and_log(&action);
        match action {
            StoreAction::StartCiPolling => self.middleware_start_ci().await,
            StoreAction::StopCiPolling => self.middleware_stop(PollKind::Ci).await,
            StoreAction::CiUpdated(_) => self.middleware_ci_updated(action).await,
            StoreAction::StartIngestPolling => self.middleware_start_ingest().await,
            StoreAction::StopIngestPolling => self.middleware_stop(PollKind::Ingest).await,
            StoreAction::StartHydrationPolling => self.middleware_start_hydration().await,
            StoreAction::StopHydrationPolling => self.middleware_stop(PollKind::Hydration).await,
            StoreAction::Reactions(a) => self.middleware_reactions(a),
            StoreAction::Roster(a) => self.middleware_roster(a),
            StoreAction::StartUpdateGithub { mutation_id } => {
                self.middleware_start_update_github(mutation_id).await
            }
            StoreAction::StopUpdateGithub { mutation_id } => {
                self.middleware_stop_update_github(mutation_id).await
            }
            StoreAction::MutationConfirmed { mutation_id } => {
                self.middleware_mutation_confirmed(mutation_id).await
            }
            StoreAction::MutationRolledBack {
                mutation_id,
                server_story,
            } => {
                self.middleware_mutation_rolled_back(mutation_id, server_story.map(|b| *b))
                    .await;
            }
        }
    }

    fn middleware_reactions(&self, action: ReactionsAction) {
        let outcome = apply_reactions(&self.reactions_store, &action);
        if outcome != StoreReduceOutcome::Changed {
            return;
        }
        let channel = match &action {
            ReactionsAction::ReactionToggled { channel, .. } => channel.clone(),
        };
        let aggregated = self.reactions_store.aggregated_for_channel(&channel);
        let listeners = {
            let l = self
                .reactions_listeners
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            l.clone()
        };
        for listener in listeners {
            listener.on_changed(&channel, &aggregated);
        }
    }

    fn middleware_roster(&self, action: RosterAction) {
        let outcome = apply_roster(&self.roster_store, &action);
        if outcome != StoreReduceOutcome::Changed {
            return;
        }
        let members = self.roster_store.members();
        let listeners = {
            let l = self
                .roster_listeners
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            l.clone()
        };
        for listener in listeners {
            listener.on_changed(&members);
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

    async fn middleware_ci_updated(&self, action: StoreAction) {
        let outcome = self.ci_store.apply(&action);
        if outcome == CiReduceOutcome::Changed {
            let snap = match action {
                StoreAction::CiUpdated(s) => s,
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
