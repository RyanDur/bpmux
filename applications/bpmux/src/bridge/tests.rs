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

#[test]
fn build_meta_always_includes_core_fields() {
    let meta = build_notification_meta(
        "owner:human",
        "2026-04-15T20:00:00Z",
        "broadcast",
        None,
        None,
    );
    assert_eq!(
        meta.get("from").and_then(|v| v.as_str()),
        Some("owner:human")
    );
    assert_eq!(
        meta.get("ts").and_then(|v| v.as_str()),
        Some("2026-04-15T20:00:00Z")
    );
    assert_eq!(
        meta.get("channel").and_then(|v| v.as_str()),
        Some("broadcast")
    );
    assert_eq!(meta.get("source").and_then(|v| v.as_str()), Some("bpmux"));
}

#[test]
fn build_meta_omits_optional_keys_when_none() {
    let meta = build_notification_meta("owner", "t", "broadcast", None, None);
    assert!(
        !meta.contains_key("addressee"),
        "addressee must be omitted when None"
    );
    assert!(
        !meta.contains_key("message_type"),
        "message_type must be omitted when None"
    );
}

#[test]
fn build_meta_includes_addressee_when_present() {
    let meta = build_notification_meta("owner", "t", "broadcast", Some("dev"), None);
    assert_eq!(meta.get("addressee").and_then(|v| v.as_str()), Some("dev"));
}

#[test]
fn build_meta_includes_message_type_when_present() {
    let meta = build_notification_meta("owner", "t", "broadcast", None, Some("insight"));
    assert_eq!(
        meta.get("message_type").and_then(|v| v.as_str()),
        Some("insight")
    );
}

#[test]
fn build_meta_never_contains_null_values() {
    let cases = [
        build_notification_meta("r", "t", "c", None, None),
        build_notification_meta("r", "t", "c", Some("dev"), None),
        build_notification_meta("r", "t", "c", None, Some("chat")),
        build_notification_meta("r", "t", "c", Some("dev"), Some("chat")),
    ];
    for meta in &cases {
        for (key, value) in meta {
            assert!(!value.is_null(), "meta[{key}] must not be null");
        }
    }
}

#[test]
fn validate_meta_accepts_payload_with_no_nulls() {
    let meta = build_notification_meta("owner", "t", "broadcast", Some("dev"), Some("chat"));
    assert!(validate_meta(&meta).is_ok());
}

#[test]
fn validate_meta_rejects_payload_with_null_value() {
    let mut meta = serde_json::Map::new();
    meta.insert("from".to_string(), serde_json::json!("owner"));
    meta.insert("addressee".to_string(), serde_json::Value::Null);
    let err = validate_meta(&meta).expect_err("should reject null");
    assert!(err.contains("addressee"));
    assert!(err.contains("null"));
}

#[test]
fn validate_meta_rejects_payload_with_null_in_any_position() {
    let keys = ["from", "ts", "channel", "source", "addressee", "topic"];
    for null_key in keys {
        let mut meta = serde_json::Map::new();
        for k in keys {
            if k == null_key {
                meta.insert(k.to_string(), serde_json::Value::Null);
            } else {
                meta.insert(k.to_string(), serde_json::json!("value"));
            }
        }
        let result = validate_meta(&meta);
        assert!(
            result.is_err(),
            "validator must catch null at key {null_key}"
        );
    }
}

#[test]
fn validate_meta_accepts_empty_map() {
    let meta = serde_json::Map::new();
    assert!(validate_meta(&meta).is_ok());
}

#[test]
fn build_meta_mixed_presence_only_includes_present_keys() {
    let meta = build_notification_meta("owner", "t", "dev", Some("dev"), Some("chat"));
    assert!(meta.contains_key("addressee"));
    assert!(meta.contains_key("message_type"));
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

use super::tools::{format_share_marker, parse_share_scope, pick_most_recent_visible_chat};

#[test]
fn parse_share_scope_all_becomes_none() {
    assert_eq!(parse_share_scope("all"), Ok(None));
}

#[test]
fn parse_share_scope_single_role_wraps_in_vec() {
    assert_eq!(parse_share_scope("dev"), Ok(Some(vec!["dev".into()])));
}

#[test]
fn parse_share_scope_comma_list_splits_and_trims() {
    assert_eq!(
        parse_share_scope("dev, pm ,  designer"),
        Ok(Some(vec!["dev".into(), "pm".into(), "designer".into()]))
    );
}

#[test]
fn parse_share_scope_rejects_empty_and_whitespace_only() {
    assert!(parse_share_scope("").is_err());
    assert!(parse_share_scope(",").is_err());
    assert!(parse_share_scope(" , ").is_err());
}

#[test]
fn format_share_marker_form1_includes_source_scope() {
    let out = format_share_marker("dev", Some("pm"), "coordinate on story X");
    assert_eq!(out, "[Shared by dev from pm thread] coordinate on story X");
}

#[test]
fn format_share_marker_form2_omits_source_scope() {
    let out = format_share_marker("dev", None, "fresh thought");
    assert_eq!(out, "[Shared by dev] fresh thought");
}

#[test]
fn pick_most_recent_visible_chat_skips_non_chat_types() {
    let messages = vec![
        serde_json::json!({
            "from": "dev",
            "text": "status update",
            "message_type": "activity",
        }),
        serde_json::json!({
            "from": "pm",
            "text": "the real one",
        }),
    ];
    let picked = pick_most_recent_visible_chat(&messages).expect("chat in list");
    assert_eq!(
        picked.get("text").and_then(|v| v.as_str()),
        Some("the real one")
    );
}

#[test]
fn pick_most_recent_visible_chat_returns_none_when_only_system_and_activity() {
    let messages = vec![
        serde_json::json!({
            "from": "bpmux",
            "text": "session started",
            "message_type": "system",
        }),
        serde_json::json!({
            "from": "dev",
            "text": "available",
            "message_type": "activity",
        }),
    ];
    assert!(pick_most_recent_visible_chat(&messages).is_none());
}

#[test]
fn pick_most_recent_visible_chat_treats_null_message_type_as_chat() {
    let messages = vec![serde_json::json!({
        "from": "owner",
        "text": "broadcast hello",
        "message_type": serde_json::Value::Null,
    })];
    let picked = pick_most_recent_visible_chat(&messages).expect("null == chat");
    assert_eq!(
        picked.get("text").and_then(|v| v.as_str()),
        Some("broadcast hello")
    );
}

mod ingest_poll_loop_tests {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use crate::board::RefreshDriver;
    use crate::board::events::BoardEvent;
    use crate::board::orchestrator::{BoxedFuture, SyncStatus};
    use crate::board::sync::SyncError;
    use crate::bridge::run_ingest_poll_loop;

    struct CountingDriver {
        refresh_all: AtomicUsize,
        drain_pending: AtomicUsize,
        fail_after: Option<usize>,
        drain_returns_n: usize,
        since_args: Mutex<Vec<Option<chrono::DateTime<chrono::Utc>>>>,
    }

    impl CountingDriver {
        fn new() -> Self {
            Self {
                refresh_all: AtomicUsize::new(0),
                drain_pending: AtomicUsize::new(0),
                fail_after: None,
                drain_returns_n: 0,
                since_args: Mutex::new(Vec::new()),
            }
        }
        fn failing_after(n: usize) -> Self {
            Self {
                refresh_all: AtomicUsize::new(0),
                drain_pending: AtomicUsize::new(0),
                fail_after: Some(n),
                drain_returns_n: 0,
                since_args: Mutex::new(Vec::new()),
            }
        }
        fn draining_n_per_tick(n: usize) -> Self {
            Self {
                refresh_all: AtomicUsize::new(0),
                drain_pending: AtomicUsize::new(0),
                fail_after: None,
                drain_returns_n: n,
                since_args: Mutex::new(Vec::new()),
            }
        }
    }

    impl RefreshDriver for CountingDriver {
        fn refresh_story(
            &self,
            _n: u64,
        ) -> BoxedFuture<'_, Result<Option<crate::board::Story>, SyncError>> {
            Box::pin(async { Ok(None) })
        }
        fn refresh_all(&self) -> BoxedFuture<'_, Result<(), SyncError>> {
            let prev = self.refresh_all.fetch_add(1, Ordering::SeqCst);
            let should_fail = self.fail_after.is_some_and(|n| prev >= n);
            Box::pin(async move {
                if should_fail {
                    Err(SyncError::Other("simulated".into()))
                } else {
                    Ok(())
                }
            })
        }
        fn refresh_all_since(
            &self,
            since: Option<chrono::DateTime<chrono::Utc>>,
        ) -> BoxedFuture<'_, Result<(), SyncError>> {
            self.since_args
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(since);
            self.refresh_all()
        }
        fn mutate_sync(&self, _e: BoardEvent) -> Result<(), SyncError> {
            Ok(())
        }
        fn drain_one_by_id(
            &self,
            _id: crate::board::MutationId,
        ) -> BoxedFuture<'_, crate::board::DrainOneOutcome> {
            Box::pin(async { crate::board::DrainOneOutcome::NotFound })
        }
        fn apply_server_rollback(&self, _story: crate::board::Story) -> BoxedFuture<'_, ()> {
            Box::pin(async {})
        }
        fn snapshot_pending_for_mutation(&self, _id: crate::board::MutationId) -> Option<u64> {
            None
        }
        fn issue_lock(
            &self,
            _issue: u64,
        ) -> BoxedFuture<'_, std::sync::Arc<tokio::sync::Mutex<()>>> {
            Box::pin(async { std::sync::Arc::new(tokio::sync::Mutex::new(())) })
        }
        fn snapshot_status(&self) -> SyncStatus {
            SyncStatus::default()
        }
        fn status_handle(&self) -> std::sync::Arc<std::sync::Mutex<SyncStatus>> {
            std::sync::Arc::new(std::sync::Mutex::new(SyncStatus::default()))
        }
        fn acknowledge_dropped(&self) -> usize {
            0
        }
        fn drain_pending(&self) -> BoxedFuture<'_, Result<usize, SyncError>> {
            self.drain_pending.fetch_add(1, Ordering::SeqCst);
            let n = self.drain_returns_n;
            Box::pin(async move { Ok(n) })
        }
        fn reconcile_dual_state_labels(&self) -> BoxedFuture<'_, Result<usize, SyncError>> {
            Box::pin(async { Ok(0) })
        }
        fn fetch_body(&self, _n: u64) -> BoxedFuture<'_, Result<String, SyncError>> {
            Box::pin(async { Ok(String::new()) })
        }
        fn fetch_free_comments(
            &self,
            _n: u64,
        ) -> BoxedFuture<'_, Result<Vec<crate::board::FreeComment>, SyncError>> {
            Box::pin(async { Ok(Vec::new()) })
        }
        fn fetch_gate_comments(
            &self,
            _n: u64,
        ) -> BoxedFuture<'_, Result<Vec<crate::board::GateComment>, SyncError>> {
            Box::pin(async { Ok(Vec::new()) })
        }
        fn create_story(
            &self,
            _title: String,
            _body: String,
            _initial_labels: Vec<String>,
        ) -> BoxedFuture<'_, Result<u64, SyncError>> {
            Box::pin(async { Ok(0) })
        }
        fn story(&self, _number: u64) -> Option<crate::board::Story> {
            None
        }
        fn all_stories(&self) -> Vec<crate::board::Story> {
            Vec::new()
        }
        fn store(&self) -> Arc<crate::board::BoardStore> {
            Arc::new(crate::board::BoardStore::new())
        }
        fn update_body(
            &self,
            _number: u64,
            _body: String,
        ) -> BoxedFuture<'_, Result<(), SyncError>> {
            Box::pin(async { Ok(()) })
        }
        fn update_title(
            &self,
            _number: u64,
            _title: String,
        ) -> BoxedFuture<'_, Result<(), SyncError>> {
            Box::pin(async { Ok(()) })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn refresh_all_fires_on_each_interval_tick() {
        let driver = Arc::new(CountingDriver::new());
        let driver_dyn: Arc<dyn RefreshDriver> = driver.clone();
        let handle = tokio::spawn(run_ingest_poll_loop(
            driver_dyn,
            Duration::from_millis(50),
            None,
        ));
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(driver.refresh_all.load(Ordering::SeqCst), 0);
        tokio::time::sleep(Duration::from_millis(250)).await;
        let count = driver.refresh_all.load(Ordering::SeqCst);
        assert_eq!(
            count, 5,
            "exactly 5 ticks fire in a 275ms window at 50ms interval under the virtual clock, saw {count}"
        );
        handle.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn loop_survives_refresh_errors() {
        let driver = Arc::new(CountingDriver::failing_after(1));
        let driver_dyn: Arc<dyn RefreshDriver> = driver.clone();
        let handle = tokio::spawn(run_ingest_poll_loop(
            driver_dyn,
            Duration::from_millis(30),
            None,
        ));
        tokio::time::sleep(Duration::from_millis(200)).await;
        let count = driver.refresh_all.load(Ordering::SeqCst);
        assert_eq!(
            count, 6,
            "loop keeps ticking across errors — exactly 6 ticks in a 200ms window at 30ms interval, saw {count}"
        );
        assert!(!handle.is_finished(), "task must not terminate on error");
        handle.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn ingest_loop_first_tick_passes_none_then_subsequent_ticks_pass_prior_start_time() {
        let driver = Arc::new(CountingDriver::new());
        let driver_dyn: Arc<dyn RefreshDriver> = driver.clone();
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
        let driver = Arc::new(CountingDriver::failing_after(1));
        let driver_dyn: Arc<dyn RefreshDriver> = driver.clone();
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
    async fn drain_loop_fires_drain_pending_on_each_tick() {
        use crate::bridge::run_drain_loop;
        use tempfile::TempDir;
        let dir = TempDir::new().expect("tmp");
        let (tx, _rx) = tokio::sync::broadcast::channel(16);
        let session = Arc::new(crate::session::Session::new(
            dir.path().to_path_buf(),
            "test".to_string(),
            tx,
        ));
        let driver = Arc::new(CountingDriver::new());
        let driver_dyn: Arc<dyn RefreshDriver> = driver.clone();
        let handle = tokio::spawn(run_drain_loop(
            driver_dyn,
            session,
            Duration::from_millis(50),
        ));
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(driver.drain_pending.load(Ordering::SeqCst), 0);
        tokio::time::sleep(Duration::from_millis(250)).await;
        let count = driver.drain_pending.load(Ordering::SeqCst);
        assert_eq!(
            count, 5,
            "exactly 5 drain ticks in a 275ms window at 50ms interval under the virtual clock, saw {count}"
        );
        handle.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn drain_loop_heartbeats_sync_status_each_tick() {
        use crate::bridge::run_drain_loop;
        use tempfile::TempDir;
        let dir = TempDir::new().expect("tmp");
        let (tx, mut rx) = tokio::sync::broadcast::channel(16);
        let session = Arc::new(crate::session::Session::new(
            dir.path().to_path_buf(),
            "test".to_string(),
            tx,
        ));
        let driver = Arc::new(CountingDriver::draining_n_per_tick(1));
        let driver_dyn: Arc<dyn RefreshDriver> = driver.clone();
        let handle = tokio::spawn(run_drain_loop(
            driver_dyn,
            session,
            Duration::from_millis(50),
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
                Some(crate::message::StateDeltaPayload::SyncPoll { .. })
            ),
            "the per-tick heartbeat is a sync_poll state-delta carrying the full status; got: {received:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn drain_loop_heartbeats_even_when_drain_returns_zero() {
        use crate::bridge::run_drain_loop;
        use tempfile::TempDir;
        let dir = TempDir::new().expect("tmp");
        let (tx, mut rx) = tokio::sync::broadcast::channel(16);
        let session = Arc::new(crate::session::Session::new(
            dir.path().to_path_buf(),
            "test".to_string(),
            tx,
        ));
        let driver = Arc::new(CountingDriver::new());
        let driver_dyn: Arc<dyn RefreshDriver> = driver.clone();
        let handle = tokio::spawn(run_drain_loop(
            driver_dyn,
            session,
            Duration::from_millis(50),
        ));
        let received = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("heartbeat must fire on a zero-drain tick — the frontend stale-watch depends on it with no frontend poll")
            .expect("broadcast channel still open");
        handle.abort();
        assert!(
            matches!(
                received.state_delta,
                Some(crate::message::StateDeltaPayload::SyncPoll { .. })
            ),
            "zero-drain ticks still heartbeat sync_poll; got: {received:?}"
        );
    }
}
