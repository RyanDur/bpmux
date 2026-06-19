use super::*;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::board::MutationId;

impl<T: CiPollFetch> Orchestrator<T> {
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
}
