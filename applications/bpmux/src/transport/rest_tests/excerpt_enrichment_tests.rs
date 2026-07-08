#![allow(unused_imports)]

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use httpmock::{Method, MockServer};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use super::{
    board_author, board_issue, board_router, board_test_session, post_json, reaction_toggle,
    response_json, tracker_for,
};

use super::{init_session, seed_messages, seed_mixed_messages, test_app};
use crate::hub;
use crate::message::MessageType;
use crate::transport::rest::*;

#[test]
fn truncate_excerpt_returns_body_unchanged_when_short() {
    let out = truncate_excerpt("thanks!", 40);
    assert_eq!(out, "thanks!");
    assert!(!out.contains('…'));
}

#[test]
fn truncate_excerpt_cuts_on_word_boundary_and_appends_ellipsis() {
    let long = "this is a fairly long message that definitely exceeds forty chars for sure";
    let out = truncate_excerpt(long, 40);
    assert!(
        out.ends_with('…'),
        "truncated excerpt must carry the ellipsis marker: {out}"
    );
    let before = out.trim_end_matches('…');
    assert!(
        !before.ends_with(' '),
        "trim_end strips trailing spaces before ellipsis: {out}"
    );
    assert!(
        out.chars().count() <= 41,
        "output stays near max_len after truncation: {out} ({})",
        out.chars().count()
    );
}

#[test]
fn truncate_excerpt_collapses_newlines_to_single_line() {
    let out = truncate_excerpt("line one\nline two", 40);
    assert!(
        !out.contains('\n'),
        "newlines must collapse so the synthesized insight stays single-line: {out}"
    );
    assert!(out.contains("line one") && out.contains("line two"));
}

#[test]
fn find_message_excerpt_matches_by_ts_and_from() {
    use chrono::TimeZone;
    let ts = chrono::Utc
        .with_ymd_and_hms(2026, 4, 23, 11, 30, 0)
        .unwrap();
    let msgs = vec![
        crate::message::Message {
            from: "dev:claude-code".to_string(),
            text: "merged #377".to_string(),
            ts,
            ..crate::message::Message::default()
        },
        crate::message::Message {
            from: "pm:claude-code".to_string(),
            text: "nice".to_string(),
            ts: ts + chrono::Duration::seconds(1),
            ..crate::message::Message::default()
        },
    ];
    let key = format!("{}|dev:claude-code", ts.to_rfc3339());
    let excerpt = find_message_excerpt(&msgs, &key, 40);
    assert_eq!(excerpt.as_deref(), Some("merged #377"));
}

#[test]
fn find_message_excerpt_returns_none_on_lookup_failure() {
    assert!(find_message_excerpt(&[], "2026-04-23T00:00:00Z|dev", 40).is_none());
    assert!(find_message_excerpt(&[], "malformed-key", 40).is_none());
    assert!(find_message_excerpt(&[], "|", 40).is_none());
}

#[test]
fn message_scope_from_key_returns_the_reacted_messages_participants() {
    use chrono::TimeZone;
    let ts = chrono::Utc
        .with_ymd_and_hms(2026, 4, 23, 11, 30, 0)
        .unwrap();
    let msgs = vec![crate::message::Message {
        from: "dev:claude-code".to_string(),
        text: "scoped to dev+pm".to_string(),
        ts,
        to: Some(vec!["dev".to_string(), "pm".to_string()]),
        ..crate::message::Message::default()
    }];
    let key = format!("{}|dev:claude-code", ts.to_rfc3339());
    let mut scope = message_scope_from_key(&msgs, &key).expect("a scoped message has a scope");
    scope.sort();
    assert_eq!(
        scope,
        vec!["dev".to_string(), "pm".to_string()],
        "a reaction inherits the reacted message's participants (sender ∪ recipients) so it never leaks to a non-participant"
    );
}

#[test]
fn message_scope_from_key_adds_the_sender_so_reactions_reach_the_author() {
    use chrono::TimeZone;
    let ts = chrono::Utc
        .with_ymd_and_hms(2026, 4, 23, 11, 30, 0)
        .unwrap();
    let msgs = vec![crate::message::Message {
        from: "owner:electron".to_string(),
        text: "owner to dev+pm".to_string(),
        ts,
        to: Some(vec!["dev".to_string(), "pm".to_string()]),
        ..crate::message::Message::default()
    }];
    let key = format!("{}|owner:electron", ts.to_rfc3339());
    let mut scope = message_scope_from_key(&msgs, &key).expect("scope");
    scope.sort();
    assert_eq!(
        scope,
        vec!["dev".to_string(), "owner".to_string(), "pm".to_string()]
    );
}

#[test]
fn message_scope_from_key_is_none_for_a_broadcast_so_its_reaction_stays_public() {
    use chrono::TimeZone;
    let ts = chrono::Utc
        .with_ymd_and_hms(2026, 4, 23, 11, 30, 0)
        .unwrap();
    let msgs = vec![crate::message::Message {
        from: "dev:claude-code".to_string(),
        text: "public".to_string(),
        ts,
        to: None,
        ..crate::message::Message::default()
    }];
    let key = format!("{}|dev:claude-code", ts.to_rfc3339());
    assert!(
        message_scope_from_key(&msgs, &key).is_none(),
        "a reaction to a broadcast stays a broadcast — no scope to inherit"
    );
}

#[test]
fn build_reaction_event_text_with_excerpt_wraps_in_typographic_quotes() {
    let body = reaction_toggle("owner", "👍", "2026-04-23T11:30:00Z|dev:claude-code");
    let text = build_reaction_event_text_with_excerpt(&body, true, Some("merged #377"));
    assert!(
        text.contains("\u{201C}merged #377\u{201D}"),
        "excerpt must wrap in typographic quotes: {text}"
    );
    assert!(text.contains("dev") && text.contains("👍"));
}

#[test]
fn build_reaction_event_text_with_excerpt_falls_back_when_excerpt_none() {
    let body = reaction_toggle("owner", "👍", "2026-04-23T11:30:00Z|dev:claude-code");
    let with_none = build_reaction_event_text_with_excerpt(&body, true, None);
    assert_eq!(with_none, "owner reacted 👍 to dev's message");
    assert!(!with_none.contains('\u{201C}'));
}

#[tokio::test]
async fn post_reaction_cascade_threads_excerpt_into_broadcast_event_text() {
    use chrono::TimeZone;
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let hub = Arc::new(hub::Hub::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx.clone(),
    ));

    let target_ts = chrono::Utc
        .with_ymd_and_hms(2026, 4, 23, 11, 30, 0)
        .unwrap();
    let target_msg = crate::message::Message {
        from: "dev:claude-code".to_string(),
        text: "merged #377 into main with the auth-cookie fix".to_string(),
        ts: target_ts,
        ..crate::message::Message::default()
    };
    hub.broadcast(&target_msg).expect("seed target message");

    let orch = std::sync::Arc::new(crate::store::Store::new(
        hub.clone(),
        None,
        crate::ci::CiStateStore::new(),
        tokio_util::sync::CancellationToken::new(),
    ));
    let router = axum::Router::new()
        .route("/api/reactions", axum::routing::post(post_reaction))
        .layer(axum::Extension(orch))
        .with_state(hub.clone());

    let message_key = format!("{}|dev:claude-code", target_ts.to_rfc3339());
    let toggle_body = format!(
        r#"{{"channel":"broadcast","message_key":"{message_key}","emoji":"👍","user":"owner"}}"#
    );
    let request = Request::builder()
        .method("POST")
        .uri("/api/reactions")
        .header("Content-Type", "application/json")
        .body(Body::from(toggle_body))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let messages = hub.read_messages().expect("read messages");
    let reaction_event = messages
        .iter()
        .find(|m| {
            m.from == "bpmux"
                && m.message_type == MessageType::Insight
                && m.text.contains("reacted")
        })
        .expect("synthesized reaction event must broadcast to hub log");

    assert!(
        reaction_event
            .text
            .contains("owner reacted 👍 to dev's message"),
        "#387 base shape preserved end-to-end: {}",
        reaction_event.text
    );
    assert!(
        reaction_event.text.contains("\u{201C}merged #377"),
        "#400 excerpt threaded through cascade in typographic quotes: {}",
        reaction_event.text
    );
    assert!(
        reaction_event.text.ends_with('\u{201D}'),
        "closing typographic quote terminates the excerpt: {}",
        reaction_event.text
    );
}

#[tokio::test]
async fn post_reaction_cascade_falls_back_to_no_excerpt_when_target_absent() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let hub = Arc::new(hub::Hub::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx.clone(),
    ));

    let orch = std::sync::Arc::new(crate::store::Store::new(
        hub.clone(),
        None,
        crate::ci::CiStateStore::new(),
        tokio_util::sync::CancellationToken::new(),
    ));
    let router = axum::Router::new()
        .route("/api/reactions", axum::routing::post(post_reaction))
        .layer(axum::Extension(orch))
        .with_state(hub.clone());

    let toggle_body = r#"{"channel":"broadcast","message_key":"2026-04-23T11:30:00+00:00|dev:claude-code","emoji":"👍","user":"owner"}"#;
    let request = Request::builder()
        .method("POST")
        .uri("/api/reactions")
        .header("Content-Type", "application/json")
        .body(Body::from(toggle_body))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let messages = hub.read_messages().expect("read messages");
    let reaction_event = messages
        .iter()
        .find(|m| {
            m.from == "bpmux"
                && m.message_type == MessageType::Insight
                && m.text.contains("reacted")
        })
        .expect("reaction event broadcasts even when lookup fails");

    assert_eq!(
        reaction_event.text, "owner reacted 👍 to dev's message",
        "lookup-fail cascade preserves #387 pre-excerpt shape byte-for-byte"
    );
    assert!(
        !reaction_event.text.contains('\u{201C}'),
        "no typographic quote when excerpt absent: {}",
        reaction_event.text
    );
}

#[tokio::test]
async fn get_ci_returns_unknown_when_store_empty() {
    use crate::ci::CiStateStore;
    let store = CiStateStore::new();
    let router = axum::Router::new()
        .route(
            "/api/ci",
            axum::routing::get(crate::transport::rest::get_ci),
        )
        .layer(axum::Extension(store));
    let request = Request::builder()
        .method("GET")
        .uri("/api/ci")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["conclusion"], "unknown");
    assert!(
        json["last_updated_at"].is_null(),
        "pre-first-poll: last_updated_at is null per AC; got {}",
        json["last_updated_at"]
    );
    assert_eq!(
        json["poll_interval_secs"],
        crate::transport::ci_watcher::POLL_INTERVAL.as_secs(),
        "AC #3: poll_interval_secs surfaces ci_watcher contract bound"
    );
    assert_eq!(
        json["backoff_cap_secs"],
        crate::transport::ci_watcher::MAX_BACKOFF.as_secs(),
        "AC #3: backoff_cap_secs surfaces ci_watcher contract bound"
    );
}

#[tokio::test]
async fn get_ci_returns_cached_state_with_last_updated_at_after_write() {
    use crate::ci::CiStateReport;
    use crate::ci::CiStateStore;
    let store = CiStateStore::new();
    let report = CiStateReport {
        conclusion: "success",
        last_run_id: Some(42),
        branch: "main",
        url: "https://github.com/acme/bpmux/actions".to_string(),
        head_sha: Some("abc1234".to_string()),
        display_title: Some("test".to_string()),
        last_updated_at: None,
        last_successful_poll_at: None,
        poll_interval_secs: 60,
        backoff_cap_secs: 3600,
        workflows: Vec::new(),
    };
    store.write(report);
    let router = axum::Router::new()
        .route(
            "/api/ci",
            axum::routing::get(crate::transport::rest::get_ci),
        )
        .layer(axum::Extension(store));
    let request = Request::builder()
        .method("GET")
        .uri("/api/ci")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["conclusion"], "success");
    assert_eq!(json["last_run_id"], 42);
    assert!(
        json["last_updated_at"].is_string(),
        "post-write: last_updated_at is RFC3339 string per AC; got {}",
        json["last_updated_at"]
    );
    let ts = json["last_updated_at"].as_str().unwrap();
    assert!(
        chrono::DateTime::parse_from_rfc3339(ts).is_ok(),
        "last_updated_at parses as RFC3339: {ts}"
    );
}

#[tokio::test]
async fn get_ci_does_not_hit_github_when_store_has_cached_state() {
    use crate::ci::CiStateReport;
    use crate::ci::CiStateStore;
    let store = CiStateStore::new();
    store.write(CiStateReport {
        conclusion: "failure",
        last_run_id: Some(7),
        branch: "main",
        url: "https://github.com/acme/bpmux/actions".to_string(),
        head_sha: Some("def5678".to_string()),
        display_title: Some("flaky test".to_string()),
        last_updated_at: None,
        last_successful_poll_at: None,
        poll_interval_secs: 60,
        backoff_cap_secs: 3600,
        workflows: Vec::new(),
    });
    let router = axum::Router::new()
        .route(
            "/api/ci",
            axum::routing::get(crate::transport::rest::get_ci),
        )
        .layer(axum::Extension(store));
    let start = std::time::Instant::now();
    let request = Request::builder()
        .method("GET")
        .uri("/api/ci")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    let elapsed = start.elapsed();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "get_ci must read cache in <50ms (no GH call); took {elapsed:?}"
    );
}

#[tokio::test]
async fn post_ci_rerun_returns_503_when_tracker_missing() {
    let none_tracker: TrackerHandle = None;
    let router = axum::Router::new()
        .route(
            "/api/ci/rerun/{run_id}",
            axum::routing::post(crate::transport::rest::post_ci_rerun),
        )
        .layer(axum::Extension(none_tracker));
    let response = post_json(router, "/api/ci/rerun/42", "").await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response_json(response).await;
    assert_eq!(body["error"], "tracker not configured");
}

#[tokio::test]
async fn post_ci_rerun_returns_success_when_tracker_post_succeeds() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST)
            .path("/repos/acme/bpmux/actions/runs/123/rerun-failed-jobs");
        then.status(201);
    });
    let tracker: TrackerHandle = Some(Arc::new(tracker_for(&server)));
    let router = axum::Router::new()
        .route(
            "/api/ci/rerun/{run_id}",
            axum::routing::post(crate::transport::rest::post_ci_rerun),
        )
        .layer(axum::Extension(tracker));
    let response = post_json(router, "/api/ci/rerun/123", "").await;
    m.assert();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["status"], "success");
    assert_eq!(body["run_id"], 123);
}

#[tokio::test]
async fn post_ci_rerun_returns_502_when_github_upstream_errors() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::POST)
            .path("/repos/acme/bpmux/actions/runs/999/rerun-failed-jobs");
        then.status(422);
    });
    let tracker: TrackerHandle = Some(Arc::new(tracker_for(&server)));
    let router = axum::Router::new()
        .route(
            "/api/ci/rerun/{run_id}",
            axum::routing::post(crate::transport::rest::post_ci_rerun),
        )
        .layer(axum::Extension(tracker));
    let response = post_json(router, "/api/ci/rerun/999", "").await;
    m.assert();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = response_json(response).await;
    assert_eq!(body["status"], "failure");
    assert_eq!(body["run_id"], 999);
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("rerun_failed_jobs")
    );
}
