use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use ci::{MAX_BACKOFF, POLL_INTERVAL};
use hub::Hub;
use kernel::Dispatch;
use message::{CiSnapshotWire, Message, MessageType, StateDeltaPayload};
use rate_limit::RateLimitObserver;
use store::ci_watch::CiWatchDriver;
use store::{AppAction, CiAction, CiPollFetch, CiSnapshot};

pub const WATCHER_UNHEALTHY_THRESHOLD: Duration = Duration::from_secs(120);

pub struct CiWatcherDriver;

impl CiWatchDriver for CiWatcherDriver {
    fn spawn_watch(
        &self,
        dispatch: Dispatch<AppAction>,
        tracker: Arc<dyn CiPollFetch>,
        hub: Arc<Hub>,
        observer: RateLimitObserver,
        ct: CancellationToken,
    ) {
        spawn(dispatch, tracker, hub, observer, ct);
    }
}

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
        let mut last_poll_at: Option<String> = None;
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        let base_backoff = Duration::from_secs(60);
        let max_backoff = MAX_BACKOFF;
        let mut backoff = base_backoff;
        loop {
            tokio::select! {
                _ = ct.cancelled() => break,
                _ = interval.tick() => {}
            }
            let conditional = tracker
                .list_latest_run_on_main_conditional(last_etag.as_deref())
                .await;
            let entries = match conditional {
                Ok(ci::ConditionalRunEntry::NotModified) => {
                    backoff = base_backoff;
                    let at = chrono::Utc::now().to_rfc3339();
                    last_poll_at = Some(at.clone());
                    dispatch(CiAction::PollSucceeded { at, updated: false }.into());
                    broadcast_ci_poll(&hub, last_snapshot.as_ref(), last_poll_at.clone());
                    continue;
                }
                Ok(ci::ConditionalRunEntry::Modified { entries, etag }) => {
                    backoff = base_backoff;
                    last_etag = etag;
                    entries
                }
                Ok(ci::ConditionalRunEntry::Empty) => {
                    backoff = base_backoff;
                    let at = chrono::Utc::now().to_rfc3339();
                    last_poll_at = Some(at.clone());
                    dispatch(CiAction::PollSucceeded { at, updated: false }.into());
                    broadcast_ci_poll(&hub, last_snapshot.as_ref(), last_poll_at.clone());
                    continue;
                }
                Err(e) => {
                    broadcast_ci_poll(&hub, last_snapshot.as_ref(), last_poll_at.clone());
                    if let Some(decision) = rate_limit::classify_and_decide(
                        &e,
                        observer.snapshot(),
                        backoff,
                        std::time::SystemTime::now(),
                    ) {
                        diag::log_diag(&format!(
                            "ci_watcher: rate-limited; backoff signal={} wait={}s",
                            rate_limit::signal_label(decision.signal),
                            decision.wait.as_secs()
                        ));
                        tokio::select! {
                            _ = ct.cancelled() => break,
                            _ = tokio::time::sleep(decision.wait) => {}
                        }
                        backoff = (backoff * 2).min(max_backoff);
                    } else {
                        diag::log_diag(&format!("ci_watcher: fetch failed: {e}"));
                    }
                    continue;
                }
            };
            let runs: Vec<ci::RunEntry> = entries
                .into_iter()
                .map(|e| ci::RunEntry {
                    status: e.status,
                    conclusion: e.conclusion,
                    head_sha: e.head_sha,
                    display_title: e.display_title,
                    database_id: e.database_id,
                    name: e.name,
                    annotation_message: e.annotation_message,
                })
                .collect();
            let classified = ci::classify_runs(&runs);
            let current: &'static str = match classified.state {
                ci::CiState::Green => "success",
                ci::CiState::Red => "failure",
                ci::CiState::Running => "in_progress",
                ci::CiState::QuotaExhausted => "quota_exhausted",
                ci::CiState::Unknown => "unknown",
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
            let at = chrono::Utc::now().to_rfc3339();
            last_poll_at = Some(at.clone());
            dispatch(CiAction::PollSucceeded { at, updated: true }.into());
            broadcast_ci_poll(&hub, last_snapshot.as_ref(), last_poll_at.clone());
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
        url: Some(ci::github_actions_url()),
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
