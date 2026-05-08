use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::board::adapters::github::GitHubSourceAdapter;
use crate::github::RateLimitObserver;
use crate::session;
use crate::transport::ci_state_store::CiStateStore;
use crate::transport::ci_watcher;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PollKind {
    Ci,
}

pub enum PollAction {
    StartCi,
    Stop(PollKind),
}

pub struct PollingControl {
    session: Arc<session::Session>,
    tracker: Option<Arc<GitHubSourceAdapter>>,
    ci_state_store: CiStateStore,
    parent_ct: CancellationToken,
    rate_limit_observer: RateLimitObserver,
    tokens: Mutex<HashMap<PollKind, CancellationToken>>,
}

impl PollingControl {
    pub fn new(
        session: Arc<session::Session>,
        tracker: Option<Arc<GitHubSourceAdapter>>,
        ci_state_store: CiStateStore,
        parent_ct: CancellationToken,
    ) -> Self {
        let rate_limit_observer = tracker
            .as_ref()
            .map(|t| t.rate_limit_observer())
            .unwrap_or_default();
        Self {
            session,
            tracker,
            ci_state_store,
            parent_ct,
            rate_limit_observer,
            tokens: Mutex::new(HashMap::new()),
        }
    }

    pub fn rate_limit_observer(&self) -> &RateLimitObserver {
        &self.rate_limit_observer
    }

    pub async fn dispatch(&self, action: PollAction) {
        match action {
            PollAction::StartCi => self.start_ci().await,
            PollAction::Stop(kind) => self.stop(kind).await,
        }
    }

    pub async fn is_running(&self, kind: PollKind) -> bool {
        let tokens = self.tokens.lock().await;
        tokens.contains_key(&kind)
    }

    async fn start_ci(&self) {
        let Some(tracker) = self.tracker.as_ref().cloned() else {
            return;
        };
        let mut tokens = self.tokens.lock().await;
        if tokens.contains_key(&PollKind::Ci) {
            return;
        }
        let local_ct = CancellationToken::new();
        let task_ct = CancellationToken::new();
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
        ci_watcher::spawn(
            Arc::clone(&self.session),
            tracker,
            self.ci_state_store.clone(),
            task_ct,
        );
        tokens.insert(PollKind::Ci, local_ct);
    }

    async fn stop(&self, kind: PollKind) {
        let mut tokens = self.tokens.lock().await;
        if let Some(token) = tokens.remove(&kind) {
            token.cancel();
        }
    }
}
