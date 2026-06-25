use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::store::ci::CiReduceOutcome;
use crate::store::{CiAction, PollKind, Store};

impl Store {
    pub(crate) async fn dispatch_ci(self: &Arc<Self>, action: CiAction) {
        match action {
            CiAction::StartPolling => self.middleware_start_ci().await,
            CiAction::StopPolling => self.middleware_stop(PollKind::Ci).await,
            CiAction::Updated(_) => self.middleware_ci_updated(action).await,
        }
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

    pub fn broadcast_ci_poll(&self, last_successful_poll_at: Option<String>) {
        let snap = self.ci_store.snapshot();
        let wire = crate::message::CiSnapshotWire {
            state: snap
                .as_ref()
                .map(|s| s.state)
                .unwrap_or(crate::ci::CiState::Unknown),
            run_id: snap.as_ref().and_then(|s| s.run_id),
            head_sha: snap.as_ref().and_then(|s| s.head_sha.clone()),
            display_title: snap.as_ref().and_then(|s| s.display_title.clone()),
            last_successful_poll_at,
            url: Some(crate::ci::github_actions_url()),
        };
        let msg = crate::message::Message {
            from: "bpmux".to_string(),
            text: String::new(),
            ts: chrono::Utc::now(),
            message_type: crate::message::MessageType::StateDelta,
            state_delta: Some(crate::message::StateDeltaPayload::CiPoll { snapshot: wire }),
            ..crate::message::Message::default()
        };
        self.session.broadcast_ephemeral(&msg);
    }
}
