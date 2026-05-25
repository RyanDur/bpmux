use super::*;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::board::MutationId;
use crate::orchestrator::stores::{
    ReactionsAction, RosterAction, StoreReduceOutcome, apply_reactions, apply_roster,
};

impl<T: CiPollFetch> Orchestrator<T> {
    pub async fn dispatch(self: &Arc<Self>, action: OrchestratorAction) {
        validate_and_log(&action);
        match action {
            OrchestratorAction::StartCiPolling => self.middleware_start_ci().await,
            OrchestratorAction::StopCiPolling => self.middleware_stop(PollKind::Ci).await,
            OrchestratorAction::CiUpdated(_) => self.middleware_ci_updated(action).await,
            OrchestratorAction::StartIngestPolling => self.middleware_start_ingest().await,
            OrchestratorAction::StopIngestPolling => self.middleware_stop(PollKind::Ingest).await,
            OrchestratorAction::StartHydrationPolling => self.middleware_start_hydration().await,
            OrchestratorAction::StopHydrationPolling => {
                self.middleware_stop(PollKind::Hydration).await
            }
            OrchestratorAction::Reactions(a) => self.middleware_reactions(a),
            OrchestratorAction::Roster(a) => self.middleware_roster(a),
            OrchestratorAction::StartUpdateGithub { mutation_id } => {
                self.middleware_start_update_github(mutation_id).await
            }
            OrchestratorAction::StopUpdateGithub { mutation_id } => {
                self.middleware_stop_update_github(mutation_id).await
            }
            OrchestratorAction::MutationConfirmed { mutation_id } => {
                self.middleware_mutation_confirmed(mutation_id).await
            }
            OrchestratorAction::MutationRolledBack {
                mutation_id,
                server_story,
            } => {
                self.middleware_mutation_rolled_back(mutation_id, server_story)
                    .await;
            }
        }
    }

    async fn middleware_start_update_github(self: &Arc<Self>, mutation_id: MutationId) {
        let Some(refresh) = self.refresh.as_ref().cloned() else {
            return;
        };
        let local_ct = {
            let mut tokens = self.mutation_tokens.lock().await;
            if tokens.contains_key(&mutation_id) {
                return;
            }
            let ct = CancellationToken::new();
            tokens.insert(mutation_id, ct.clone());
            ct
        };
        let this = Arc::clone(self);
        let parent_ct = self.parent_ct.clone();
        tokio::spawn(async move {
            let outcome = tokio::select! {
                _ = parent_ct.clone().cancelled_owned() => return,
                _ = local_ct.clone().cancelled_owned() => return,
                r = refresh.drain_one_by_id(mutation_id) => r,
            };
            match outcome {
                crate::board::DrainOneOutcome::Confirmed => {
                    this.middleware_mutation_confirmed(mutation_id).await;
                }
                crate::board::DrainOneOutcome::RolledBack { server_story } => {
                    this.middleware_mutation_rolled_back(mutation_id, server_story.map(|b| *b))
                        .await;
                }
                crate::board::DrainOneOutcome::TransientError(_)
                | crate::board::DrainOneOutcome::NotFound => {
                    this.middleware_stop_update_github(mutation_id).await;
                }
            }
        });
    }

    async fn middleware_stop_update_github(self: &Arc<Self>, mutation_id: MutationId) {
        let mut tokens = self.mutation_tokens.lock().await;
        if let Some(ct) = tokens.remove(&mutation_id) {
            ct.cancel();
        }
    }

    async fn middleware_mutation_confirmed(self: &Arc<Self>, mutation_id: MutationId) {
        let mut tokens = self.mutation_tokens.lock().await;
        tokens.remove(&mutation_id);
    }

    async fn middleware_mutation_rolled_back(
        self: &Arc<Self>,
        mutation_id: MutationId,
        server_story: Option<crate::board::domain::Story>,
    ) {
        {
            let mut tokens = self.mutation_tokens.lock().await;
            tokens.remove(&mutation_id);
        }
        if let (Some(refresh), Some(story)) = (self.refresh.as_ref(), server_story) {
            refresh.apply_server_rollback(story).await;
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
        crate::transport::ci_watcher::spawn::<T>(
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

    async fn middleware_ci_updated(&self, action: OrchestratorAction) {
        let outcome = self.ci_store.apply(&action);
        if outcome == CiReduceOutcome::Changed {
            let snap = match action {
                OrchestratorAction::CiUpdated(s) => s,
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
