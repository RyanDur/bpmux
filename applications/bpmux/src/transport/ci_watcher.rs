use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::ci::CiStateStore;
use crate::hub::Hub;
use crate::message::{CiSnapshotWire, Message, MessageType, StateDeltaPayload};
use crate::rate_limit::RateLimitObserver;
use crate::store::{CiAction, CiPollFetch, CiSnapshot};
use kernel::Dispatch;

pub const POLL_INTERVAL: Duration = Duration::from_secs(10);
pub const MAX_BACKOFF: Duration = Duration::from_secs(3600);
pub const WATCHER_UNHEALTHY_THRESHOLD: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    Changed { new_conclusion: &'static str },
    Same,
    NoData,
}

pub type Observation = (Option<u64>, &'static str);

pub fn classify_transition(prev: Option<Observation>, current: Observation) -> Transition {
    match prev {
        Some(p) if p == current => Transition::Same,
        _ => Transition::Changed {
            new_conclusion: current.1,
        },
    }
}

pub fn spawn<A>(
    dispatch: Dispatch<A>,
    tracker: Arc<dyn CiPollFetch>,
    ci_state_store: CiStateStore,
    hub: Arc<Hub>,
    observer: RateLimitObserver,
    ct: CancellationToken,
) where
    A: 'static,
    CiAction: Into<A>,
{
    tokio::spawn(async move {
        let mut last_etag: Option<String> = None;
        let mut last_snapshot: Option<CiSnapshot> = None;
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        let base_backoff = Duration::from_secs(60);
        let max_backoff = MAX_BACKOFF;
        let mut backoff = base_backoff;
        let poll_time =
            |store: &CiStateStore| store.last_successful_poll_at().map(|t| t.to_rfc3339());
        loop {
            tokio::select! {
                _ = ct.cancelled() => break,
                _ = interval.tick() => {}
            }
            let conditional = tracker
                .list_latest_run_on_main_conditional(last_etag.as_deref())
                .await;
            let entries = match conditional {
                Ok(crate::bridge::tracker::ConditionalRunEntry::NotModified) => {
                    backoff = base_backoff;
                    ci_state_store.mark_poll_succeeded();
                    broadcast_ci_poll(&hub, last_snapshot.as_ref(), poll_time(&ci_state_store));
                    continue;
                }
                Ok(crate::bridge::tracker::ConditionalRunEntry::Modified { entries, etag }) => {
                    backoff = base_backoff;
                    last_etag = etag;
                    entries
                }
                Ok(crate::bridge::tracker::ConditionalRunEntry::Empty) => {
                    backoff = base_backoff;
                    ci_state_store.mark_poll_succeeded();
                    broadcast_ci_poll(&hub, last_snapshot.as_ref(), poll_time(&ci_state_store));
                    continue;
                }
                Err(e) => {
                    broadcast_ci_poll(&hub, last_snapshot.as_ref(), poll_time(&ci_state_store));
                    if let Some(decision) = crate::rate_limit::classify_and_decide(
                        &e,
                        observer.snapshot(),
                        backoff,
                        std::time::SystemTime::now(),
                    ) {
                        crate::diag::log_diag(&format!(
                            "ci_watcher: rate-limited; backoff signal={} wait={}s",
                            crate::rate_limit::signal_label(decision.signal),
                            decision.wait.as_secs()
                        ));
                        tokio::select! {
                            _ = ct.cancelled() => break,
                            _ = tokio::time::sleep(decision.wait) => {}
                        }
                        backoff = (backoff * 2).min(max_backoff);
                    } else {
                        crate::diag::log_diag(&format!("ci_watcher: fetch failed: {e}"));
                    }
                    continue;
                }
            };
            let runs: Vec<crate::ci::RunEntry> = entries
                .into_iter()
                .map(|e| crate::ci::RunEntry {
                    status: e.status,
                    conclusion: e.conclusion,
                    head_sha: e.head_sha,
                    display_title: e.display_title,
                    database_id: e.database_id,
                    name: e.name,
                    annotation_message: e.annotation_message,
                })
                .collect();
            let classified = crate::ci::classify_runs(&runs);
            let current: &'static str = match classified.state {
                crate::ci::CiState::Green => "success",
                crate::ci::CiState::Red => "failure",
                crate::ci::CiState::Running => "in_progress",
                crate::ci::CiState::QuotaExhausted => "quota_exhausted",
                crate::ci::CiState::Unknown => "unknown",
            };
            let snapshot = CiSnapshot {
                state: classified.state,
                run_id: classified.run_id,
                head_sha: classified.sha.clone(),
                display_title: classified.title.clone(),
                conclusion: current,
                workflows: classified.workflows.clone(),
            };
            dispatch(CiAction::Updated(snapshot.clone()).into());
            last_snapshot = Some(snapshot);
            ci_state_store.mark_poll_succeeded();
            broadcast_ci_poll(&hub, last_snapshot.as_ref(), poll_time(&ci_state_store));
        }
    });
}

fn broadcast_ci_poll(
    hub: &Hub,
    snapshot: Option<&CiSnapshot>,
    last_successful_poll_at: Option<String>,
) {
    let wire = CiSnapshotWire {
        state: snapshot.map(|s| s.state.into()).unwrap_or_default(),
        run_id: snapshot.and_then(|s| s.run_id),
        head_sha: snapshot.and_then(|s| s.head_sha.clone()),
        display_title: snapshot.and_then(|s| s.display_title.clone()),
        last_successful_poll_at,
        url: Some(crate::ci::github_actions_url()),
    };
    let msg = Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: MessageType::StateDelta,
        state_delta: Some(StateDeltaPayload::CiPoll { snapshot: wire }),
        ..Message::default()
    };
    hub.broadcast_ephemeral(&msg);
}
