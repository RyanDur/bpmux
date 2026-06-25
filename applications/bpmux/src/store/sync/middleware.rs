use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::product_management::board::{DrainOneOutcome, MutationId};
use crate::product_management::story::Story;
use crate::store::{PollKind, Store, SyncAction};

impl Store {
    pub(crate) async fn dispatch_sync(self: &Arc<Self>, action: SyncAction) {
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

    pub(super) async fn middleware_start_update_github(self: &Arc<Self>, mutation_id: MutationId) {
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
                DrainOneOutcome::Confirmed => {
                    this.middleware_mutation_confirmed(mutation_id).await;
                }
                DrainOneOutcome::RolledBack { server_story } => {
                    this.middleware_mutation_rolled_back(mutation_id, server_story.map(|b| *b))
                        .await;
                }
                DrainOneOutcome::TransientError(_) | DrainOneOutcome::NotFound => {
                    this.middleware_stop_update_github(mutation_id).await;
                }
            }
        });
    }

    pub(super) async fn middleware_stop_update_github(self: &Arc<Self>, mutation_id: MutationId) {
        let mut tokens = self.mutation_tokens.lock().await;
        if let Some(ct) = tokens.remove(&mutation_id) {
            ct.cancel();
        }
    }

    pub(super) async fn middleware_mutation_confirmed(self: &Arc<Self>, mutation_id: MutationId) {
        let mut tokens = self.mutation_tokens.lock().await;
        tokens.remove(&mutation_id);
    }

    pub(super) async fn middleware_mutation_rolled_back(
        self: &Arc<Self>,
        mutation_id: MutationId,
        server_story: Option<Story>,
    ) {
        {
            let mut tokens = self.mutation_tokens.lock().await;
            tokens.remove(&mutation_id);
        }
        if let (Some(refresh), Some(story)) = (self.refresh.as_ref(), server_story) {
            refresh.apply_server_rollback(story).await;
        }
    }
}
