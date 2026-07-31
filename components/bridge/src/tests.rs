#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn resolve_bridge_name_uses_given_name_when_provided() {
    assert_eq!(resolve_bridge_name("dev"), "dev");
    assert_eq!(resolve_bridge_name("pm"), "pm");
    assert_eq!(resolve_bridge_name("designer"), "designer");
}

#[test]
fn resolve_bridge_name_generates_agent_suffix_for_default() {
    let name = resolve_bridge_name("claude-code");
    assert!(
        name.starts_with("agent-"),
        "expected agent-XXXX, got: {name}"
    );
    assert_eq!(name.len(), "agent-".len() + 4);
}

#[test]
fn resolve_bridge_name_generates_agent_suffix_when_empty() {
    let name = resolve_bridge_name("");
    assert!(
        name.starts_with("agent-"),
        "empty name should auto-generate, got: {name}"
    );
}

#[test]
fn resolve_bridge_name_generates_agent_suffix_for_unexpanded_placeholder() {
    let name = resolve_bridge_name("${BPMUX_ROLE}");
    assert!(
        name.starts_with("agent-"),
        "unexpanded placeholder should auto-generate, got: {name}"
    );
}

use super::tools::curate_message;

#[test]
fn curate_message_preserves_addressee_when_present() {
    let raw = serde_json::json!({
        "from": "dev",
        "text": "hello pm",
        "ts": "2026-04-15T21:00:00Z",
        "addressee": "pm",
    });
    let out = curate_message(&raw).expect("should curate");
    assert_eq!(out.get("addressee").and_then(|v| v.as_str()), Some("pm"));
    assert_eq!(
        out.get("ts").and_then(|v| v.as_str()),
        Some("2026-04-15T21:00:00Z")
    );
}

#[test]
fn curate_message_preserves_topic_when_present() {
    let raw = serde_json::json!({
        "from": "dev",
        "text": "teaching point",
        "ts": "2026-04-15T21:00:00Z",
        "topic": "teaching",
    });
    let out = curate_message(&raw).expect("should curate");
    assert_eq!(out.get("topic").and_then(|v| v.as_str()), Some("teaching"));
}

#[test]
fn curate_message_preserves_message_type_when_present() {
    let raw = serde_json::json!({
        "from": "dev",
        "text": "reasoning trace",
        "ts": "2026-04-15T21:00:00Z",
        "message_type": "insight",
    });
    let out = curate_message(&raw).expect("should curate");
    assert_eq!(
        out.get("message_type").and_then(|v| v.as_str()),
        Some("insight")
    );
}

#[test]
fn curate_message_broadcast_has_null_addressee() {
    let raw = serde_json::json!({
        "from": "dev",
        "text": "broadcast",
        "ts": "2026-04-15T21:00:00Z",
    });
    let out = curate_message(&raw).expect("should curate");
    assert!(out.get("addressee").is_some(), "addressee key must exist");
    assert!(
        out["addressee"].is_null(),
        "broadcast addressee must be null"
    );
}

#[test]
fn curate_message_addressee_visible_across_recipients() {
    let raw = serde_json::json!({
        "from": "owner",
        "text": "work on this dev",
        "ts": "2026-04-15T21:00:00Z",
        "addressee": "dev",
    });
    let out = curate_message(&raw).expect("should curate");
    assert_eq!(out.get("addressee").and_then(|v| v.as_str()), Some("dev"));
}

#[test]
fn curate_message_returns_none_when_required_fields_missing() {
    let raw = serde_json::json!({
        "text": "no sender",
        "ts": "2026-04-15T21:00:00Z",
    });
    assert!(
        curate_message(&raw).is_none(),
        "missing `from` must skip the message"
    );

    let raw = serde_json::json!({
        "from": "dev",
        "ts": "2026-04-15T21:00:00Z",
    });
    assert!(
        curate_message(&raw).is_none(),
        "missing `text` must skip the message"
    );
}

#[test]
fn curate_message_broadcast_distinguishable_from_addressed() {
    let broadcast = serde_json::json!({
        "from": "owner", "text": "hi team", "ts": "t",
    });
    let addressed = serde_json::json!({
        "from": "owner", "text": "hi dev", "ts": "t", "addressee": "dev",
    });
    let broadcast_out = curate_message(&broadcast).expect("should curate");
    let addressed_out = curate_message(&addressed).expect("should curate");
    assert!(broadcast_out["addressee"].is_null());
    assert_eq!(addressed_out["addressee"].as_str(), Some("dev"));
    assert_ne!(broadcast_out, addressed_out, "outputs must differ in shape");
}

#[test]
fn curate_message_preserves_to_array_for_privacy_scoped() {
    let raw = serde_json::json!({
        "from": "dev",
        "text": "private",
        "ts": "t",
        "to": ["pm", "designer"],
    });
    let out = curate_message(&raw).expect("should curate");
    let to = out.get("to").and_then(|v| v.as_array()).expect("to array");
    assert_eq!(to.len(), 2);
    assert_eq!(to[0].as_str(), Some("pm"));
    assert_eq!(to[1].as_str(), Some("designer"));
}

mod poll_loop_tests {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use crate::run_ingest_poll_loop;
    use crate::{IngestTicker, SyncStatusSource};
    use source_sync::SyncStatus;
    use sync::BoxedFuture;
    use sync::SyncError;

    struct CountingIngest {
        ticks: AtomicUsize,
        flushes: AtomicUsize,
        fail_after: Option<usize>,
        since_args: Mutex<Vec<Option<chrono::DateTime<chrono::Utc>>>>,
    }

    impl CountingIngest {
        fn new() -> Self {
            Self {
                ticks: AtomicUsize::new(0),
                flushes: AtomicUsize::new(0),
                fail_after: None,
                since_args: Mutex::new(Vec::new()),
            }
        }
        fn failing_after(n: usize) -> Self {
            Self {
                ticks: AtomicUsize::new(0),
                flushes: AtomicUsize::new(0),
                fail_after: Some(n),
                since_args: Mutex::new(Vec::new()),
            }
        }
    }

    impl IngestTicker for CountingIngest {
        fn ingest(
            &self,
            since: Option<chrono::DateTime<chrono::Utc>>,
        ) -> BoxedFuture<'_, Result<(), SyncError>> {
            self.since_args
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(since);
            let prev = self.ticks.fetch_add(1, Ordering::SeqCst);
            let should_fail = self.fail_after.is_some_and(|n| prev >= n);
            let result = if should_fail {
                Err(SyncError::Other("simulated".into()))
            } else {
                Ok(())
            };
            Box::pin(async move { result })
        }

        fn flush_pending_writes(&self) -> BoxedFuture<'_, Result<usize, SyncError>> {
            self.flushes.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(0) })
        }
    }

    struct StubStatusSource;

    impl SyncStatusSource for StubStatusSource {
        fn status(&self) -> SyncStatus {
            SyncStatus::default()
        }
    }

    #[tokio::test(start_paused = true)]
    async fn refresh_all_fires_on_each_interval_tick() {
        let driver = Arc::new(CountingIngest::new());
        let driver_dyn: Arc<dyn IngestTicker> = driver.clone();
        let handle = tokio::spawn(run_ingest_poll_loop(
            driver_dyn,
            Duration::from_millis(50),
            None,
        ));
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(driver.ticks.load(Ordering::SeqCst), 0);
        tokio::time::sleep(Duration::from_millis(250)).await;
        let count = driver.ticks.load(Ordering::SeqCst);
        assert_eq!(
            count, 5,
            "exactly 5 ticks fire in a 275ms window at 50ms interval under the virtual clock, saw {count}"
        );
        handle.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn ingest_poll_flushes_pending_writes_on_success_but_not_on_a_failed_poll() {
        let ok_driver = Arc::new(CountingIngest::new());
        let ok_dyn: Arc<dyn IngestTicker> = ok_driver.clone();
        let ok_handle = tokio::spawn(run_ingest_poll_loop(
            ok_dyn,
            Duration::from_millis(50),
            None,
        ));
        tokio::time::sleep(Duration::from_millis(120)).await;
        ok_handle.abort();
        assert!(
            ok_driver.flushes.load(Ordering::SeqCst) >= 1,
            "a successful ingest poll proves GitHub is reachable, so the loop flushes writes stuck behind an earlier transient failure — the event-driven retry that replaces the removed drain timer; saw {} flush(es) over {} tick(s)",
            ok_driver.flushes.load(Ordering::SeqCst),
            ok_driver.ticks.load(Ordering::SeqCst)
        );

        let failing_driver = Arc::new(CountingIngest::failing_after(0));
        let failing_dyn: Arc<dyn IngestTicker> = failing_driver.clone();
        let failing_handle = tokio::spawn(run_ingest_poll_loop(
            failing_dyn,
            Duration::from_millis(50),
            None,
        ));
        tokio::time::sleep(Duration::from_millis(120)).await;
        failing_handle.abort();
        assert_eq!(
            failing_driver.flushes.load(Ordering::SeqCst),
            0,
            "a failed ingest poll means GitHub is unreachable — a flush would be pointless, so the retry is tied strictly to the success (recovery) event; saw {} tick(s)",
            failing_driver.ticks.load(Ordering::SeqCst)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn loop_survives_refresh_errors() {
        let driver = Arc::new(CountingIngest::failing_after(1));
        let driver_dyn: Arc<dyn IngestTicker> = driver.clone();
        let handle = tokio::spawn(run_ingest_poll_loop(
            driver_dyn,
            Duration::from_millis(30),
            None,
        ));
        tokio::time::sleep(Duration::from_millis(200)).await;
        let count = driver.ticks.load(Ordering::SeqCst);
        assert_eq!(
            count, 6,
            "loop keeps ticking across errors — exactly 6 ticks in a 200ms window at 30ms interval, saw {count}"
        );
        assert!(!handle.is_finished(), "task must not terminate on error");
        handle.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn ingest_loop_first_tick_passes_none_then_subsequent_ticks_pass_prior_start_time() {
        let driver = Arc::new(CountingIngest::new());
        let driver_dyn: Arc<dyn IngestTicker> = driver.clone();
        let handle = tokio::spawn(run_ingest_poll_loop(
            driver_dyn,
            Duration::from_millis(50),
            None,
        ));
        tokio::time::sleep(Duration::from_millis(220)).await;
        handle.abort();
        let captured = driver
            .since_args
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        assert_eq!(
            captured.len(),
            4,
            "exactly 4 ticks fire in a 220ms window at 50ms interval under the virtual clock; since args: {captured:?}"
        );
        assert!(
            captured[0].is_none(),
            "first poll uses since=None (full poll on startup): {captured:?}"
        );
        assert!(
            captured[1..].iter().all(|s| s.is_some()),
            "subsequent polls pass since=Some(prior-start-ts): {captured:?}"
        );
        let monotonic = captured
            .iter()
            .filter_map(|s| s.as_ref())
            .collect::<Vec<_>>()
            .windows(2)
            .all(|w| w[0] <= w[1]);
        assert!(
            monotonic,
            "since timestamps advance monotonically: {captured:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn ingest_loop_does_not_advance_since_on_failure() {
        let driver = Arc::new(CountingIngest::failing_after(1));
        let driver_dyn: Arc<dyn IngestTicker> = driver.clone();
        let handle = tokio::spawn(run_ingest_poll_loop(
            driver_dyn,
            Duration::from_millis(40),
            None,
        ));
        tokio::time::sleep(Duration::from_millis(220)).await;
        handle.abort();
        let captured = driver
            .since_args
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let with_some: Vec<_> = captured.iter().filter(|s| s.is_some()).collect();
        assert_eq!(
            with_some.len(),
            4,
            "exactly 4 since-bearing ticks (tick 1 passes None, ticks 2-5 pass Some) in a 220ms window at 40ms interval; saw {captured:?}"
        );
        assert_eq!(
            with_some[0], with_some[1],
            "after failure, since stays at last successful tick: {captured:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn sync_status_heartbeat_broadcasts_syncpoll_each_tick() {
        use crate::run_sync_status_heartbeat;
        use tempfile::TempDir;
        let dir = TempDir::new().expect("tmp");
        let (tx, mut rx) = tokio::sync::broadcast::channel(16);
        let hub = Arc::new(hub::Hub::new(
            dir.path().to_path_buf(),
            "test".to_string(),
            tx,
        ));
        let source: Arc<dyn SyncStatusSource> = Arc::new(StubStatusSource);
        let handle = tokio::spawn(run_sync_status_heartbeat(
            source,
            hub,
            Duration::from_millis(50),
            std::sync::Arc::new(|| true),
        ));
        let received = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("a sync-status heartbeat must broadcast within 500ms of the first tick")
            .expect("broadcast channel still open");
        handle.abort();
        assert_eq!(received.from, "bpmux");
        assert!(
            matches!(
                received.state_delta,
                Some(message::StateDeltaPayload::SyncPoll { .. })
            ),
            "the per-tick heartbeat is a sync_poll state-delta carrying the full status; got: {received:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn sync_status_heartbeat_fires_every_tick_so_the_frontend_stale_watch_advances_without_a_client_poll()
     {
        use crate::run_sync_status_heartbeat;
        use tempfile::TempDir;
        let dir = TempDir::new().expect("tmp");
        let (tx, mut rx) = tokio::sync::broadcast::channel(16);
        let hub = Arc::new(hub::Hub::new(
            dir.path().to_path_buf(),
            "test".to_string(),
            tx,
        ));
        let source: Arc<dyn SyncStatusSource> = Arc::new(StubStatusSource);
        let handle = tokio::spawn(run_sync_status_heartbeat(
            source,
            hub,
            Duration::from_millis(50),
            std::sync::Arc::new(|| true),
        ));
        let first = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("first heartbeat must fire within 500ms")
            .expect("broadcast channel still open");
        let second = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("a second heartbeat must fire on the next tick with no client poll")
            .expect("broadcast channel still open");
        handle.abort();
        assert!(
            matches!(
                first.state_delta,
                Some(message::StateDeltaPayload::SyncPoll { .. })
            ) && matches!(
                second.state_delta,
                Some(message::StateDeltaPayload::SyncPoll { .. })
            ),
            "the heartbeat is periodic — every tick emits a sync_poll regardless of drain activity (there is no drain now), so a quiescent system still advances the wall-clock stale-watch; got first={first:?} second={second:?}"
        );
    }
}
