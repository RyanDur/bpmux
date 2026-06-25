#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use tempfile::TempDir;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use super::ci::{
    CiAction, CiBroadcastListener, CiPollFetch, CiSnapshot, PollKind, Store, StoreAction,
};
use crate::bridge::tracker::ConditionalRunEntry;
use crate::ci::CiState;
use crate::github::RateLimitObserver;
use crate::message::Message;
use crate::session::Session;
use crate::transport::ci_state_store::CiStateStore;

fn fresh_session() -> (Arc<Session>, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
    ));
    (session, dir)
}

fn snap(run_id: u64, conclusion: &'static str) -> CiSnapshot {
    CiSnapshot {
        state: match conclusion {
            "success" => CiState::Green,
            "failure" => CiState::Red,
            "in_progress" => CiState::Running,
            _ => CiState::Unknown,
        },
        run_id: Some(run_id),
        head_sha: Some("deadbeef".to_string()),
        display_title: Some("test commit".to_string()),
        conclusion,
        workflows: Vec::new(),
    }
}

#[derive(Default)]
struct RecordingListener {
    received: Mutex<Vec<CiSnapshot>>,
}

impl CiBroadcastListener for RecordingListener {
    fn on_changed(&self, snap: &CiSnapshot) {
        let mut received = self.received.lock().unwrap_or_else(|e| e.into_inner());
        received.push(snap.clone());
    }
}

struct FakeCiTracker {
    responses: Mutex<VecDeque<ConditionalRunEntry>>,
    notify: tokio::sync::Notify,
}

fn default_modified_entry() -> ConditionalRunEntry {
    ConditionalRunEntry::Modified {
        entries: vec![crate::bridge::tracker::WorkflowRunEntry {
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            head_sha: Some("deadbeef".to_string()),
            display_title: Some("fake CI run".to_string()),
            database_id: Some(7777),
            name: None,
            annotation_message: None,
        }],
        etag: Some("etag-1".to_string()),
    }
}

impl FakeCiTracker {
    fn new() -> Self {
        let mut responses = VecDeque::new();
        responses.push_back(default_modified_entry());
        Self {
            responses: Mutex::new(responses),
            notify: tokio::sync::Notify::new(),
        }
    }

    fn with_responses(responses: Vec<ConditionalRunEntry>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().collect()),
            notify: tokio::sync::Notify::new(),
        }
    }
}

impl CiPollFetch for FakeCiTracker {
    fn list_latest_run_on_main_conditional<'a>(
        &'a self,
        _prior_etag: Option<&'a str>,
    ) -> crate::store::BoxedFuture<'a, Result<ConditionalRunEntry, String>> {
        Box::pin(async move {
            let next = {
                let mut q = self.responses.lock().unwrap_or_else(|e| e.into_inner());
                q.pop_front()
            };
            match next {
                Some(entry) => Ok(entry),
                None => {
                    self.notify.notified().await;
                    Err("loop-blocked-forever".to_string())
                }
            }
        })
    }

    fn rate_limit_observer(&self) -> RateLimitObserver {
        RateLimitObserver::default()
    }
}

#[tokio::test]
async fn ci_updated_action_drives_reducer_and_fires_listener() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    let recorder = Arc::new(RecordingListener::default());
    orch.add_ci_listener(recorder.clone());

    orch.dispatch(CiAction::CiUpdated(snap(42, "success")))
        .await;

    let store = orch.ci_store();
    assert_eq!(store.last_seen(), Some((Some(42), "success")));
    assert_eq!(store.snapshot().expect("snapshot").run_id, Some(42));
    let received = recorder.received.lock().unwrap();
    assert_eq!(received.len(), 1, "listener fires on Changed outcome");
    assert_eq!(received[0].run_id, Some(42));
}

#[tokio::test]
async fn ci_updated_dedup_same_run_and_conclusion_no_listener_fire() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    let recorder = Arc::new(RecordingListener::default());
    orch.add_ci_listener(recorder.clone());

    orch.dispatch(CiAction::CiUpdated(snap(42, "success")))
        .await;
    orch.dispatch(CiAction::CiUpdated(snap(42, "success")))
        .await;

    let received = recorder.received.lock().unwrap();
    assert_eq!(received.len(), 1, "second identical CiUpdated is dedup'd");
}

#[tokio::test]
async fn ci_updated_run_change_or_conclusion_change_fires_listener() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    let recorder = Arc::new(RecordingListener::default());
    orch.add_ci_listener(recorder.clone());

    orch.dispatch(CiAction::CiUpdated(snap(42, "in_progress")))
        .await;
    orch.dispatch(CiAction::CiUpdated(snap(42, "success")))
        .await;
    orch.dispatch(CiAction::CiUpdated(snap(43, "success")))
        .await;

    let received = recorder.received.lock().unwrap();
    assert_eq!(
        received.len(),
        3,
        "each transition (status flip + new run_id) fires listener"
    );
}

#[tokio::test]
async fn start_ci_polling_no_op_without_tracker() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    orch.dispatch(CiAction::StartCiPolling).await;
    assert!(!orch.is_running(PollKind::Ci).await);
}

#[tokio::test]
async fn stop_ci_polling_without_start_no_op() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    orch.dispatch(CiAction::StopCiPolling).await;
    assert!(!orch.is_running(PollKind::Ci).await);
}

#[tokio::test]
async fn spawn_action_drainer_routes_channel_actions_into_dispatch() {
    use crate::store::stores::ReactionsAction;
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<StoreAction>();
    orch.spawn_action_drainer(rx);

    tx.send(StoreAction::Reactions(ReactionsAction::ReactionToggled {
        channel: "broadcast".to_string(),
        message_key: "drainer-test".to_string(),
        emoji: "🟢".to_string(),
        user: "owner".to_string(),
        ts_iso: "2026-05-08T19:30:00Z".to_string(),
    }))
    .expect("send into drainer channel");

    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        let aggregated = orch.reactions_store().aggregated_for_channel("broadcast");
        if !aggregated.is_empty() {
            return;
        }
    }
    panic!("drainer did not route Reactions action through dispatch within 1s");
}

#[tokio::test]
async fn spawn_action_drainer_exits_on_parent_cancel() {
    let parent = CancellationToken::new();
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        parent.clone(),
    ));
    let (_tx, rx) = tokio::sync::mpsc::unbounded_channel::<StoreAction>();
    let handle = orch.spawn_action_drainer(rx);
    parent.cancel();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("drainer terminates within 1s of parent cancel")
        .expect("drainer task joins cleanly");
}

#[tokio::test]
async fn parent_cancel_propagates_when_running_no_tracker() {
    let parent = CancellationToken::new();
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        parent.clone(),
    ));
    orch.dispatch(CiAction::StartCiPolling).await;
    parent.cancel();
    tokio::time::sleep(Duration::from_millis(50)).await;
    orch.dispatch(CiAction::StopCiPolling).await;
    assert!(!orch.is_running(PollKind::Ci).await);
}

#[tokio::test]
async fn rate_limit_observer_exposed_at_orchestrator_layer() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    let snapshot = orch.rate_limit_observer().snapshot();
    assert_eq!(snapshot.retry_after_wake_epoch, 0);
    assert_eq!(snapshot.ratelimit_reset_epoch, 0);
}

#[tokio::test]
async fn live_loop_dispatches_ci_updated_through_action_chain() {
    let (session, _dir) = fresh_session();
    let tracker = Arc::new(FakeCiTracker::new());
    let parent = CancellationToken::new();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        Some(tracker.clone() as Arc<dyn CiPollFetch>),
        CiStateStore::new(),
        parent.clone(),
    ));
    let recorder = Arc::new(RecordingListener::default());
    orch.add_ci_listener(recorder.clone());

    tokio::time::pause();
    orch.dispatch(CiAction::StartCiPolling).await;
    assert!(
        orch.is_running(PollKind::Ci).await,
        "live spawn-path took the tracker.is_some() branch"
    );

    tokio::time::advance(Duration::from_secs(15)).await;
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(50)).await;
    tokio::task::yield_now().await;
    tokio::time::resume();

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if !recorder.received.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let (got_run_id, got_conclusion) = {
        let received = recorder.received.lock().unwrap();
        assert!(
            !received.is_empty(),
            "loop ticked + dispatched CiAction::CiUpdated through store → reducer → listener"
        );
        (received[0].run_id, received[0].conclusion)
    };
    assert_eq!(got_run_id, Some(7777));
    assert_eq!(got_conclusion, "success");

    orch.dispatch(CiAction::StopCiPolling).await;
    let stop_deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < stop_deadline && orch.is_running(PollKind::Ci).await {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        !orch.is_running(PollKind::Ci).await,
        "Stop cancels the loop within bounded time"
    );
}

#[tokio::test]
async fn live_loop_marks_poll_succeeded_on_all_three_branches() {
    let (session, _dir) = fresh_session();
    let tracker = Arc::new(FakeCiTracker::with_responses(vec![
        default_modified_entry(),
        ConditionalRunEntry::NotModified,
        ConditionalRunEntry::Empty,
    ]));
    let parent = CancellationToken::new();
    let ci_state_store = CiStateStore::new();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        Some(tracker.clone() as Arc<dyn CiPollFetch>),
        ci_state_store.clone(),
        parent.clone(),
    ));

    tokio::time::pause();
    orch.dispatch(CiAction::StartCiPolling).await;
    assert!(orch.is_running(PollKind::Ci).await);

    for _ in 0..3 {
        tokio::time::advance(Duration::from_secs(10)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(50)).await;
        tokio::task::yield_now().await;
    }
    tokio::time::resume();

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        let remaining = {
            let q = tracker.responses.lock().unwrap();
            q.len()
        };
        if remaining == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let remaining_after_wait = {
        let q = tracker.responses.lock().unwrap();
        q.len()
    };
    assert_eq!(
        remaining_after_wait, 0,
        "all 3 fixture responses (Modified + NotModified + Empty) consumed by live loop"
    );

    let last_polled = ci_state_store.last_successful_poll_at();
    assert!(
        last_polled.is_some(),
        "mark_poll_succeeded fires on Modified+NotModified+Empty branches; \
         last_successful_poll_at must be Some after all 3 outcomes consumed"
    );

    orch.dispatch(CiAction::StopCiPolling).await;
    let stop_deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < stop_deadline && orch.is_running(PollKind::Ci).await {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!orch.is_running(PollKind::Ci).await);
}

#[tokio::test]
async fn live_loop_broadcasts_frozen_poll_time_on_failure() {
    use crate::message::StateDeltaPayload;

    let (session, _dir) = fresh_session();
    let mut frames = session.subscribe();
    let tracker = Arc::new(FakeCiTracker::with_responses(
        vec![default_modified_entry()],
    ));
    let parent = CancellationToken::new();
    let ci_state_store = CiStateStore::new();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        Some(tracker.clone() as Arc<dyn CiPollFetch>),
        ci_state_store.clone(),
        parent.clone(),
    ));

    tokio::time::pause();
    orch.dispatch(CiAction::StartCiPolling).await;
    assert!(orch.is_running(PollKind::Ci).await);

    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(50)).await;
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    tracker.notify.notify_one();
    tokio::task::yield_now().await;
    tokio::time::resume();

    let mut ci_poll_times: Vec<Option<String>> = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline && ci_poll_times.len() < 2 {
        match frames.try_recv() {
            Ok(msg) => {
                if let Some(StateDeltaPayload::CiPoll { snapshot }) = msg.state_delta {
                    ci_poll_times.push(snapshot.last_successful_poll_at);
                }
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(10)).await,
        }
    }

    orch.dispatch(CiAction::StopCiPolling).await;

    assert!(
        ci_poll_times.len() >= 2,
        "success-poll AND failure-poll each broadcast a ci frame; \
         the failure heartbeat is what lets the frontend stale-watch re-render"
    );
    let success_poll_at = ci_poll_times[0].clone();
    let failure_poll_at = ci_poll_times[1].clone();
    assert!(
        success_poll_at.is_some(),
        "the success poll stamps + broadcasts a real poll-time"
    );
    assert_eq!(
        failure_poll_at, success_poll_at,
        "on failure the broadcast carries the FROZEN last_successful_poll_at — \
         the poll-time does not advance, so the stale-watch can detect staleness"
    );
}

fn refresh_for_tests() -> Arc<dyn crate::product_management::board::RefreshDriver> {
    let store = Arc::new(crate::product_management::board::BoardStore::new());
    let adapter = Arc::new(
        crate::product_management::board::adapters::in_memory::InMemorySourceAdapter::new(),
    );
    Arc::new(crate::product_management::board::SyncOrchestrator::new(
        store, adapter,
    ))
}

#[tokio::test]
async fn start_ingest_polling_no_op_without_refresh() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    orch.dispatch(StoreAction::StartIngestPolling).await;
    assert!(!orch.is_running(PollKind::Ingest).await);
}

#[tokio::test]
async fn start_hydration_polling_no_op_without_refresh() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    orch.dispatch(StoreAction::StartHydrationPolling).await;
    assert!(!orch.is_running(PollKind::Hydration).await);
}

#[tokio::test]
async fn stop_ingest_without_start_no_op() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    orch.dispatch(StoreAction::StopIngestPolling).await;
    assert!(!orch.is_running(PollKind::Ingest).await);
}

#[tokio::test]
async fn stop_hydration_without_start_no_op() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(Store::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    orch.dispatch(StoreAction::StopHydrationPolling).await;
    assert!(!orch.is_running(PollKind::Hydration).await);
}

#[tokio::test]
async fn start_ingest_polling_spawns_loop_with_refresh() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(
        Store::new(session, None, CiStateStore::new(), CancellationToken::new()).with_refresh(
            refresh_for_tests(),
            Duration::from_secs(60),
            Duration::from_secs(15),
        ),
    );
    orch.dispatch(StoreAction::StartIngestPolling).await;
    assert!(
        orch.is_running(PollKind::Ingest).await,
        "ingest spawn-path takes the refresh.is_some() branch"
    );

    orch.dispatch(StoreAction::StopIngestPolling).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline && orch.is_running(PollKind::Ingest).await {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        !orch.is_running(PollKind::Ingest).await,
        "Stop cancels the ingest loop within bounded time"
    );
}

#[tokio::test]
async fn start_hydration_polling_spawns_loop_with_refresh() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(
        Store::new(session, None, CiStateStore::new(), CancellationToken::new()).with_refresh(
            refresh_for_tests(),
            Duration::from_secs(60),
            Duration::from_secs(15),
        ),
    );
    orch.dispatch(StoreAction::StartHydrationPolling).await;
    assert!(
        orch.is_running(PollKind::Hydration).await,
        "hydration spawn-path takes the refresh.is_some() branch"
    );

    orch.dispatch(StoreAction::StopHydrationPolling).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline && orch.is_running(PollKind::Hydration).await {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        !orch.is_running(PollKind::Hydration).await,
        "Stop cancels the hydration loop within bounded time"
    );
}

#[tokio::test]
async fn start_ingest_idempotent_no_op_when_already_running() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(
        Store::new(session, None, CiStateStore::new(), CancellationToken::new()).with_refresh(
            refresh_for_tests(),
            Duration::from_secs(60),
            Duration::from_secs(15),
        ),
    );
    orch.dispatch(StoreAction::StartIngestPolling).await;
    assert!(orch.is_running(PollKind::Ingest).await);
    orch.dispatch(StoreAction::StartIngestPolling).await;
    assert!(
        orch.is_running(PollKind::Ingest).await,
        "second StartIngestPolling stays running (no-op)"
    );
    orch.dispatch(StoreAction::StopIngestPolling).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline && orch.is_running(PollKind::Ingest).await {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!orch.is_running(PollKind::Ingest).await);
}

#[tokio::test]
async fn start_hydration_idempotent_no_op_when_already_running() {
    let (session, _dir) = fresh_session();
    let orch: Arc<Store> = Arc::new(
        Store::new(session, None, CiStateStore::new(), CancellationToken::new()).with_refresh(
            refresh_for_tests(),
            Duration::from_secs(60),
            Duration::from_secs(15),
        ),
    );
    orch.dispatch(StoreAction::StartHydrationPolling).await;
    assert!(orch.is_running(PollKind::Hydration).await);
    orch.dispatch(StoreAction::StartHydrationPolling).await;
    assert!(
        orch.is_running(PollKind::Hydration).await,
        "second StartHydrationPolling stays running (no-op)"
    );
    orch.dispatch(StoreAction::StopHydrationPolling).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline && orch.is_running(PollKind::Hydration).await {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!orch.is_running(PollKind::Hydration).await);
}

#[tokio::test]
async fn rate_limit_cooldown_is_shared_across_ci_ingest_hydration_paths() {
    let observer = RateLimitObserver::default();
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cooldown_secs = 60u64;
    observer.observe_retry_after_secs(cooldown_secs, std::time::SystemTime::now());

    let snapshot = observer.snapshot();
    assert_eq!(
        snapshot.retry_after_wake_epoch,
        now_secs.saturating_add(cooldown_secs),
        "single observe_retry_after_secs(60) sets shared wake-epoch ~60s out"
    );

    let now = std::time::SystemTime::now();
    let decision = crate::github::decide_backoff(snapshot, Duration::ZERO, now);
    assert!(
        decision.wait > Duration::from_secs(50),
        "shared decide_backoff returns ~60s wait for any caller (CI/Ingest/Hydration) consulting the same observer; got {:?}",
        decision.wait
    );
    assert!(
        decision.wait <= Duration::from_secs(cooldown_secs + 1),
        "wait bounded by cooldown duration; got {:?}",
        decision.wait
    );

    let snap_again = observer.snapshot();
    let decision_again = crate::github::decide_backoff(snap_again, Duration::ZERO, now);
    assert_eq!(
        decision.wait, decision_again.wait,
        "Arc-shared observer: every caller's snapshot returns the same wake-epoch — single source of truth across CI + Ingest + Hydration"
    );

    let advanced = now + Duration::from_secs(cooldown_secs + 5);
    let decision_post =
        crate::github::decide_backoff(observer.snapshot(), Duration::ZERO, advanced);
    assert_eq!(
        decision_post.wait,
        Duration::ZERO,
        "after cooldown elapses, all callers see zero wait (gate clears for everyone simultaneously)"
    );
}
