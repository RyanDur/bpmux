use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

use crate::product_management::board::{DrainOneOutcome, MutationId, RefreshDriver};
use crate::store::app::{AppAction, AppState};
use crate::store::kernel::{Dispatch, Listener, ListenerFuture, Reaction};

use super::SyncAction;

pub struct GithubDrain {
    refresh: Arc<dyn RefreshDriver>,
    parent_ct: CancellationToken,
    mutations: Mutex<HashMap<MutationId, CancellationToken>>,
}

impl GithubDrain {
    pub fn new(refresh: Arc<dyn RefreshDriver>, parent_ct: CancellationToken) -> Self {
        Self {
            refresh,
            parent_ct,
            mutations: Mutex::new(HashMap::new()),
        }
    }

    pub fn pending_count(&self) -> usize {
        self.mutations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    fn start(&self, id: MutationId, dispatch: Dispatch<AppAction>) {
        let mut mutations = self
            .mutations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if mutations.contains_key(&id) {
            return;
        }
        let token = self.parent_ct.child_token();
        mutations.insert(id, token.clone());
        let refresh = self.refresh.clone();
        tokio::spawn(async move {
            let outcome = tokio::select! {
                _ = token.cancelled() => return,
                drained = refresh.drain_one_by_id(id) => drained,
            };
            let action = match outcome {
                DrainOneOutcome::Confirmed => SyncAction::MutationConfirmed { mutation_id: id },
                DrainOneOutcome::RolledBack { server_story } => SyncAction::MutationRolledBack {
                    mutation_id: id,
                    server_story,
                },
                DrainOneOutcome::TransientError(_) | DrainOneOutcome::NotFound => {
                    SyncAction::StopUpdateGithub { mutation_id: id }
                }
            };
            dispatch(action.into());
        });
    }

    fn forget(&self, id: MutationId) {
        self.mutations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&id);
    }

    fn cancel(&self, id: MutationId) {
        if let Some(token) = self
            .mutations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&id)
        {
            token.cancel();
        }
    }

    fn rolled_back(
        &self,
        id: MutationId,
        server_story: Option<Box<crate::product_management::story::Story>>,
    ) -> ListenerFuture {
        self.forget(id);
        let refresh = self.refresh.clone();
        Box::pin(async move {
            if let Some(story) = server_story {
                refresh.apply_server_rollback(*story).await;
            }
        })
    }
}

impl Listener<AppState, AppAction> for GithubDrain {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        let Reaction {
            action, dispatch, ..
        } = reaction;
        let AppAction::Sync(action) = action else {
            return Box::pin(async {});
        };
        match action {
            SyncAction::StartUpdateGithub { mutation_id } => self.start(mutation_id, dispatch),
            SyncAction::StopUpdateGithub { mutation_id } => self.cancel(mutation_id),
            SyncAction::MutationConfirmed { mutation_id } => self.forget(mutation_id),
            SyncAction::MutationRolledBack {
                mutation_id,
                server_story,
            } => return self.rolled_back(mutation_id, server_story),
            _ => {}
        }
        Box::pin(async {})
    }
}
