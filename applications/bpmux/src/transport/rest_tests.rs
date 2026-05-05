#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::rest::*;
use crate::session;
use crate::session::MessageType;
use axum::body::Body;
use axum::http::Request;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use axum::http::StatusCode;

fn test_app(dir: &TempDir) -> (axum::Router, broadcast::Sender<session::Message>) {
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx.clone(),
    ));
    let router = axum::Router::new()
        .route(
            "/api/messages",
            axum::routing::get(get_messages).post(post_message),
        )
        .with_state(session);
    (router, tx)
}

fn init_session(dir: &TempDir) -> PathBuf {
    let messages_file = dir.path().join("messages-current.jsonl");
    std::fs::write(&messages_file, "").expect("create messages file");
    dir.path().to_path_buf()
}

#[tokio::test]
async fn posts_message_and_broadcasts() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app(&dir);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"from":"test:hook","text":"hello from hook"}"#,
        ))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert_eq!(msg.from, "test:hook");
    assert_eq!(msg.text, "hello from hook");
}

#[tokio::test]
async fn post_message_with_to_sets_explicit_privacy_true() {
    // Agent-originated DMs (or any POST /message with `to`) must
    // carry the same privacy flag as the TUI's /to path. Without
    // this, /share would silently widen a supposedly-private agent
    // DM because the explicit-privacy guard doesn't recognize it.
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app(&dir);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"from":"dev:claude-code","text":"shh","to":["pm"]}"#,
        ))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert!(
        msg.explicit_privacy,
        "DM with `to` filter must set explicit_privacy=true"
    );
    assert_eq!(msg.to.as_deref(), Some(["pm".to_string()].as_slice()));
}

#[tokio::test]
async fn post_message_without_to_leaves_explicit_privacy_false() {
    // Broadcasts (no `to`) are not DMs — explicit_privacy stays
    // false so /share can widen them as expected.
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app(&dir);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"from":"dev:claude-code","text":"open"}"#))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("broadcast message");
    assert!(
        !msg.explicit_privacy,
        "broadcast must not set explicit_privacy"
    );
    assert!(msg.to.is_none(), "broadcast must have no `to` filter");
}

#[tokio::test]
async fn rejects_empty_text() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, _tx) = test_app(&dir);
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"from":"test:hook","text":""}"#))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn rejects_missing_fields() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, _tx) = test_app(&dir);
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"text":"no from field"}"#))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

fn seed_messages(dir: &TempDir, count: usize) -> Arc<session::Session> {
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    for i in 0..count {
        let msg = session::Message {
            from: "user".to_string(),
            text: format!("msg {i}"),
            ts: chrono::Utc::now(),
            ..session::Message::default()
        };
        session.broadcast(&msg).expect("broadcast");
    }
    session
}

#[tokio::test]
async fn gets_messages_with_limit() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_messages(&dir, 5);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages?limit=2")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 2);
    // Most recent first
    assert_eq!(msgs[0]["text"], "msg 4");
    assert_eq!(msgs[1]["text"], "msg 3");
}

#[tokio::test]
async fn gets_messages_before_timestamp_returns_only_older() {
    // #1153: paged-history. `before=<ts>` filters to strict-less-than
    // so the client can fetch older windows by passing the oldest-
    // currently-rendered timestamp. Boundary message excluded to
    // avoid duplicates at the page-stitch.
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    // Seed 5 messages with deterministic timestamps 1-5 seconds apart.
    let base = chrono::Utc::now() - chrono::Duration::seconds(60);
    for i in 0..5 {
        let msg = session::Message {
            from: "user".to_string(),
            text: format!("msg {i}"),
            ts: base + chrono::Duration::seconds(i as i64),
            ..session::Message::default()
        };
        session.broadcast(&msg).expect("broadcast");
    }
    let boundary = base + chrono::Duration::seconds(3);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    // Use Z-suffix RFC3339 form ("...Z" instead of "...+00:00") so the
    // url-decoder doesn't interpret `+` as space. Z is the unambiguous
    // URL-safe shape callers should send.
    let boundary_z = boundary.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
    let uri = format!("/api/messages?limit=10&before={boundary_z}");
    let request = Request::builder()
        .method("GET")
        .uri(&uri)
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    // before=base+3s: msg 0/1/2 included (ts < boundary); msg 3 (ts == boundary) excluded; msg 4 excluded.
    // Reversed-newest-first → [msg 2, msg 1, msg 0].
    assert_eq!(
        msgs.len(),
        3,
        "expected msgs 0/1/2 strictly before boundary; got {msgs:?}"
    );
    assert_eq!(msgs[0]["text"], "msg 2");
    assert_eq!(msgs[1]["text"], "msg 1");
    assert_eq!(msgs[2]["text"], "msg 0");
}

#[tokio::test]
async fn gets_messages_before_invalid_timestamp_falls_back_to_no_filter() {
    // Invalid `before=<garbage>` falls back to "no filter" rather
    // than 400-erroring. Defensive: if a future client sends a
    // malformed param the endpoint shouldn't break for them.
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_messages(&dir, 3);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages?limit=10&before=not-a-timestamp")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 3);
}

#[tokio::test]
async fn gets_messages_with_offset() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_messages(&dir, 5);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages?limit=2&offset=2")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 2);
    // Reversed: [4,3,2,1,0], skip 2 -> [2,1,0], take 2 -> [2,1]
    assert_eq!(msgs[0]["text"], "msg 2");
    assert_eq!(msgs[1]["text"], "msg 1");
}

fn seed_mixed_messages(dir: &TempDir) -> Arc<session::Session> {
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    for (text, msg_type) in [
        ("hello", MessageType::Chat),
        ("busy", MessageType::Activity),
        ("world", MessageType::Chat),
        ("available", MessageType::Activity),
        ("goodbye", MessageType::Chat),
    ] {
        let msg = session::Message {
            from: "dev".to_string(),
            text: text.to_string(),
            ts: chrono::Utc::now(),
            message_type: msg_type,
            ..session::Message::default()
        };
        session.broadcast(&msg).expect("broadcast");
    }
    session
}

#[tokio::test]
async fn chat_only_excludes_activity_by_default() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_mixed_messages(&dir);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages?limit=50")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 3, "should only return chat messages");
    assert!(msgs.iter().all(|m| m["message_type"].is_null()));
}

#[tokio::test]
async fn chat_only_false_returns_all() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_mixed_messages(&dir);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages?limit=50&chat_only=false")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 5, "should return all messages");
}

#[tokio::test]
async fn message_type_filter_returns_only_matching_type() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_mixed_messages(&dir);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages?limit=50&message_type=activity")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 2, "should return only activity messages");
    assert!(
        msgs.iter().all(|m| m["message_type"] == "activity"),
        "every returned message should be activity-typed"
    );
}

#[tokio::test]
async fn defaults_to_twenty_messages() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let session = seed_messages(&dir, 25);
    let router = axum::Router::new()
        .route("/api/messages", axum::routing::get(get_messages))
        .with_state(session);
    let request = Request::builder()
        .method("GET")
        .uri("/api/messages")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(msgs.len(), 20);
}

// ---------- Classifier misfire endpoint ----------

#[tokio::test]
async fn classifier_misfire_accepts_post_with_reason() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let router = axum::Router::new()
        .route(
            "/api/classifier-misfire",
            axum::routing::post(post_classifier_misfire),
        )
        .with_state(session);
    let request = Request::builder()
        .method("POST")
        .uri("/api/classifier-misfire")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"agent":"dev","nudge_hash":"abc123","reason":"recursive match on bpmux nudge"}"#,
        ))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn classifier_misfire_accepts_post_without_reason() {
    // Per the story's acceptance criteria: reason is OPTIONAL.
    // State the observation, not the conclusion — agents don't have
    // to explain WHY a fire was wrong, just marking it as wrong is
    // valuable feedback.
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let router = axum::Router::new()
        .route(
            "/api/classifier-misfire",
            axum::routing::post(post_classifier_misfire),
        )
        .with_state(session);
    let request = Request::builder()
        .method("POST")
        .uri("/api/classifier-misfire")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"agent":"dev","nudge_hash":"xyz789"}"#))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn post_message_rejects_external_system_message_type() {
    // The auto-subscribe path in session::log mutates topic state
    // based on system messages. Accepting them from outside lets an
    // attacker manipulate topic routing. POST /message must refuse
    // message_type="system" regardless of who sent it.
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app(&dir);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"from":"attacker","text":"Created topic 'secret' with: ryan,dev","message_type":"system"}"#,
        ))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    // The rejected message must NOT reach the broadcast stream —
    // external callers must not be able to spoof internal system
    // signals (join/leave, classifier nudges) via HTTP POST.
    assert!(
        rx.try_recv().is_err(),
        "external system message must not broadcast"
    );
}

#[tokio::test]
async fn post_message_accepts_chat_message_type() {
    // Sanity: rejecting `system` must not block normal chat. This
    // pairs with the rejection test to prove the guard is
    // type-scoped, not a blanket block.
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app(&dir);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"from":"dev","text":"hello","message_type":"chat"}"#,
        ))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = rx.try_recv().expect("chat should broadcast");
    assert_eq!(msg.text, "hello");
}

#[tokio::test]
async fn post_message_accepts_default_message_type() {
    // Backward compat: omitting `message_type` defaults to Chat and
    // must still work.
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (app, tx) = test_app(&dir);
    let mut rx = tx.subscribe();
    let request = Request::builder()
        .method("POST")
        .uri("/api/messages")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"from":"dev","text":"default"}"#))
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let _ = rx.try_recv().expect("default type should broadcast");
}

// Serialize BPMUX_LOG mutation across tests so parallel envs don't
// race. Same pattern as bridge/tracker_tests.
fn bpmux_log_env_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::await_holding_lock)]
async fn classifier_misfire_escapes_adversarial_agent_input() {
    // Craft an `agent` value with newlines, quotes, and format-
    // string characters. A naive `{}` format would splice those
    // directly into the log, corrupting structure and enabling log
    // injection. The Debug `{:?}` format escapes them.
    let _guard = bpmux_log_env_lock();
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let router = axum::Router::new()
        .route(
            "/api/classifier-misfire",
            axum::routing::post(post_classifier_misfire),
        )
        .with_state(session);

    let log_dir = TempDir::new().expect("log tmpdir");
    let log_path = log_dir.path().join("bpmux.log");
    let prev = std::env::var("BPMUX_LOG").ok();
    // SAFETY: bpmux_log_env_lock guard scopes this mutation.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("BPMUX_LOG", &log_path);
    }

    let request = Request::builder()
        .method("POST")
        .uri("/api/classifier-misfire")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"agent":"evil\nMISFIRE agent=\"fake\"","nudge_hash":"{curly}"}"#,
        ))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);

    let log_contents = std::fs::read_to_string(&log_path).unwrap_or_default();

    #[allow(unsafe_code)]
    unsafe {
        match prev {
            Some(v) => std::env::set_var("BPMUX_LOG", v),
            None => std::env::remove_var("BPMUX_LOG"),
        }
    }

    // Each adversarial line can't be interpreted as a real MISFIRE
    // entry because Debug escapes the embedded newline + quotes.
    let forged_count = log_contents
        .lines()
        .filter(|l| l.contains("MISFIRE agent=\"fake\""))
        .count();
    assert_eq!(
        forged_count, 0,
        "adversarial newline + quotes must not forge a second MISFIRE line. log:\n{log_contents}"
    );
    // The real logged line must contain the escaped form of the
    // input so the record stays auditable.
    assert!(
        log_contents.contains("\\n") || log_contents.contains("\\\""),
        "Debug format should produce escaped metacharacters. log:\n{log_contents}"
    );
    assert!(
        log_contents.contains("{curly}") || log_contents.contains("\"{curly}\""),
        "nudge_hash should render through Debug format. log:\n{log_contents}"
    );
}

#[tokio::test]
async fn classifier_misfire_requires_agent_and_hash() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let router = axum::Router::new()
        .route(
            "/api/classifier-misfire",
            axum::routing::post(post_classifier_misfire),
        )
        .with_state(session);
    // Missing nudge_hash
    let request = Request::builder()
        .method("POST")
        .uri("/api/classifier-misfire")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"agent":"dev"}"#))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    // serde rejection produces 4xx, not 201
    assert_ne!(response.status(), StatusCode::CREATED);
}

// ─────────────────────────────────────────────────────────────────────
// Board + story REST endpoint tests
//
// Two axes: (a) tracker-absent path — handlers must return 503 with
// `{"error": "tracker not configured"}` so the renderer can show a
// banner rather than a blank board; (b) tracker-present path — an
// httpmock-backed OctocrabStoryTracker simulates the GitHub REST
// surface so we exercise the real HTTP client + JSON shaping.
// ─────────────────────────────────────────────────────────────────────

use crate::bridge::tracker::OctocrabStoryTracker;
use crate::transport::rest::TrackerHandle;
use httpmock::{Method, MockServer};
use serde_json::{Value, json};

fn board_test_session(
    dir: &TempDir,
) -> (Arc<session::Session>, broadcast::Sender<session::Message>) {
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "ryan".to_string(),
        tx.clone(),
    ));
    (session, tx)
}

/// Mirrors `tracker_contract_tests::author`. Octocrab's Issue requires
/// a fully-populated Author; this keeps the board REST tests self-
/// contained — duplicating ~20 lines of JSON is cheaper than exposing
/// the bridge test helpers cross-module.
fn board_author() -> Value {
    json!({
        "login": "ryan",
        "id": 1,
        "node_id": "MDQ6VXNlcjE=",
        "avatar_url": "https://example.test/avatar.png",
        "gravatar_id": "",
        "url": "https://example.test/users/ryan",
        "html_url": "https://example.test/ryan",
        "followers_url": "https://example.test/ryan/followers",
        "following_url": "https://example.test/ryan/following{/other_user}",
        "gists_url": "https://example.test/ryan/gists{/gist_id}",
        "starred_url": "https://example.test/ryan/starred{/owner}{/repo}",
        "subscriptions_url": "https://example.test/ryan/subscriptions",
        "organizations_url": "https://example.test/ryan/orgs",
        "repos_url": "https://example.test/ryan/repos",
        "events_url": "https://example.test/ryan/events{/privacy}",
        "received_events_url": "https://example.test/ryan/received_events",
        "type": "User",
        "site_admin": false
    })
}

fn board_issue(number: u64, title: &str, state: &str, body: Option<&str>) -> Value {
    json!({
        "id": number,
        "node_id": "MDU6SXNzdWUx",
        "url": format!("https://example.test/issues/{number}"),
        "repository_url": "https://example.test/repo",
        "labels_url": format!("https://example.test/issues/{number}/labels{{/name}}"),
        "comments_url": format!("https://example.test/issues/{number}/comments"),
        "events_url": format!("https://example.test/issues/{number}/events"),
        "html_url": format!("https://example.test/web/{number}"),
        "number": number,
        "state": state,
        "state_reason": null,
        "title": title,
        "body": body,
        "user": board_author(),
        "labels": [],
        "assignees": [],
        "author_association": "OWNER",
        "locked": false,
        "comments": 0,
        "created_at": "2026-04-18T12:34:56Z",
        "updated_at": "2026-04-18T12:34:56Z"
    })
}

fn tracker_for(server: &MockServer) -> crate::board::adapters::github::GitHubSourceAdapter {
    let inner = OctocrabStoryTracker::with_base_url(
        &format!("{}/", server.base_url()),
        "acme".to_string(),
        "bpmux".to_string(),
    )
    .expect("build tracker against mock");
    crate::board::adapters::github::GitHubSourceAdapter::new(inner)
}

fn board_router(session: Arc<session::Session>, tracker: TrackerHandle) -> axum::Router {
    use axum::routing::{get, post};
    let none_refresh: crate::server::TestRefreshHandle = None;
    axum::Router::new()
        .route("/api/board", get(crate::transport::rest::get_board))
        .route(
            "/api/stories/{number}",
            get(crate::transport::rest::get_story),
        )
        .route(
            "/api/stories/{number}/accept",
            post(crate::transport::rest::post_accept_story),
        )
        .route(
            "/api/stories/{number}/reject",
            post(crate::transport::rest::post_reject_story),
        )
        .with_state(session)
        .layer(axum::Extension(tracker))
        .layer(axum::Extension(none_refresh))
}

async fn get_json(router: axum::Router, uri: &str) -> axum::http::Response<Body> {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("build request");
    router.oneshot(request).await.expect("response")
}

async fn post_json(router: axum::Router, uri: &str, body: &str) -> axum::http::Response<Body> {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    router.oneshot(request).await.expect("response")
}

async fn response_json(response: axum::http::Response<Body>) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&body).expect("parse json")
}

#[tokio::test]
async fn get_board_returns_503_when_tracker_missing() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let router = board_router(session, None);

    let response = get_json(router, "/api/board").await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response_json(response).await;
    assert_eq!(body["error"], "tracker not configured");
}

#[tokio::test]
async fn get_board_returns_stories_with_fetched_at() {
    let server = MockServer::start();
    let open_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("state", "open");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![board_issue(1, "first", "open", Some("hello"))]).to_string());
    });
    let closed_mock = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues")
            .query_param("state", "closed");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![board_issue(2, "second", "closed", None)]).to_string());
    });

    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = get_json(router, "/api/board").await;
    open_mock.assert();
    closed_mock.assert();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let stories = body["stories"].as_array().expect("stories array");
    assert_eq!(stories.len(), 2);
    assert_eq!(stories[0]["number"], 1);
    assert_eq!(stories[0]["title"], "first");
    // State derivation: open issue with no workflow labels → icebox;
    // closed issue → accepted. Mirrors the renderer's bucket vocabulary.
    assert_eq!(stories[0]["state"], "icebox");
    assert_eq!(stories[0]["body"], "hello");
    // Missing body falls back to "" — renderer gets a stable string,
    // not a null surprise.
    assert_eq!(stories[1]["body"], "");
    assert_eq!(stories[1]["state"], "accepted");
    // Required shape fields per spec.
    assert!(stories[0]["labels"].is_array());
    assert!(stories[0]["assignees"].is_array());
    assert!(stories[0]["created_at"].is_string());
    assert!(
        body["fetched_at"].is_string(),
        "fetched_at timestamp required"
    );
}

#[tokio::test]
async fn get_story_returns_single_issue_with_body() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues/42");
        then.status(200)
            .header("content-type", "application/json")
            .body(board_issue(42, "answer", "open", Some("full body text")).to_string());
    });

    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = get_json(router, "/api/stories/42").await;
    m.assert();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["number"], 42);
    assert_eq!(body["title"], "answer");
    assert_eq!(body["body"], "full body text");
}

#[tokio::test]
async fn get_story_returns_404_when_github_returns_404() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/9999");
        then.status(404)
            .header("content-type", "application/json")
            .body(r#"{"message":"Not Found","documentation_url":"https://docs"}"#);
    });

    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = get_json(router, "/api/stories/9999").await;
    m.assert();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert!(body["error"].as_str().unwrap().contains("#9999 not found"));
}

#[tokio::test]
async fn get_story_returns_503_when_tracker_missing() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let router = board_router(session, None);

    let response = get_json(router, "/api/stories/1").await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn post_accept_story_returns_503_when_orchestrator_missing() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let router = board_router(session, None);

    let response = post_json(
        router,
        "/api/stories/42/accept",
        r#"{"would_file_bug":false}"#,
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn post_accept_story_400_when_would_file_bug_missing() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let server = MockServer::start();
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = post_json(router, "/api/stories/42/accept", "{}").await;
    assert!(
        response.status() == StatusCode::UNPROCESSABLE_ENTITY
            || response.status() == StatusCode::BAD_REQUEST,
        "missing required would_file_bug must 4xx; got {}",
        response.status()
    );
}

#[tokio::test]
async fn post_accept_story_400_when_would_file_bug_true_without_force() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let server = MockServer::start();
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = post_json(
        router,
        "/api/stories/42/accept",
        r#"{"would_file_bug":true}"#,
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert!(
        body["error"]
            .as_str()
            .unwrap_or("")
            .contains("reject_story"),
        "would_file_bug=true must route the user to reject_story; got {body}"
    );
}

#[tokio::test]
async fn post_reject_story_400_on_empty_note() {
    // The note field is load-bearing: it's what Ryan writes back to
    // dev on a rejection. Empty note means the rejection has no
    // actionable context — fail loud at the boundary rather than
    // posting an empty comment.
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    // Tracker present so we know the 400 comes from validation, not
    // the 503 path.
    let server = MockServer::start();
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = post_json(router, "/api/stories/7/reject", r#"{"note":""}"#).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["error"], "note must not be empty");
}

#[tokio::test]
async fn post_reject_story_400_on_whitespace_note() {
    // Whitespace-only notes are the same trap as empty — the comment
    // posted to GitHub would be a blank line. Trim-check at the
    // transport boundary so the renderer can't trivially bypass.
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let server = MockServer::start();
    let tracker = Some(Arc::new(tracker_for(&server)));
    let router = board_router(session, tracker);

    let response = post_json(router, "/api/stories/7/reject", r#"{"note":"   \n\t "}"#).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_reject_story_returns_503_when_tracker_missing() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, _tx) = board_test_session(&dir);
    let router = board_router(session, None);

    let response = post_json(router, "/api/stories/7/reject", r#"{"note":"x"}"#).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

// ── #387 reaction-event synthesis tests ──────────────────────────

fn reaction_toggle(user: &str, emoji: &str, message_key: &str) -> ReactionToggle {
    ReactionToggle {
        channel: "broadcast".to_string(),
        message_key: message_key.to_string(),
        emoji: emoji.to_string(),
        user: user.to_string(),
    }
}

#[test]
fn build_reaction_event_text_names_sender_from_message_key() {
    // message_key shape is "{ts}|{from}"; parsing extracts the
    // original sender's short-role so the synthesized event text
    // reads "X reacted 👍 to Y's message" with Y = the role the
    // message originated from. Agents can respond contextually
    // (Ryan AC4).
    let body = reaction_toggle("ryan", "👍", "2026-04-23T11:30:00Z|dev:claude-code");
    let text = build_reaction_event_text_with_excerpt(&body, true, None);
    assert!(
        text.contains("ryan") && text.contains("👍") && text.contains("dev"),
        "text should name reactor + emoji + sender: {text}"
    );
    assert!(
        text.contains("reacted") && !text.contains("unreacted"),
        "present=true should use 'reacted' verb: {text}"
    );
}

#[test]
fn build_reaction_event_text_unreact_uses_unreacted_verb() {
    // Designer 2026-04-23 11:58 UTC: "removed {emoji}" reads as
    // ambiguous ("{user} removed my reaction"). "unreacted" keeps
    // the "who did what" structure — user removes THEIR OWN
    // reaction, not someone else's.
    let body = reaction_toggle("dev", "🎉", "2026-04-23T11:30:00Z|pm:claude-code");
    let text = build_reaction_event_text_with_excerpt(&body, false, None);
    // "unreacted" naturally contains "reacted" as substring, so the
    // negative assertion checks there's no standalone "reacted"
    // word (bounded by non-letter on the left side).
    assert!(
        text.contains("unreacted"),
        "should use 'unreacted' verb: {text}"
    );
    let starts_with_reacted = text.starts_with("reacted ") || text.contains(" reacted ");
    assert!(
        !starts_with_reacted,
        "present=false must not include standalone 'reacted' word: {text}"
    );
    assert!(text.contains("dev") && text.contains("🎉") && text.contains("pm"));
}

#[test]
fn build_reaction_event_text_falls_back_without_message_key_shape() {
    // If message_key lacks the "ts|from" format, the synthesized
    // text still carries user + emoji + verb. Agents lose only
    // the "whose message" context — partial fallback, not panic.
    let body = reaction_toggle("ryan", "✨", "malformed-key");
    let text = build_reaction_event_text_with_excerpt(&body, true, None);
    assert!(text.contains("ryan") && text.contains("✨"));
    assert!(
        !text.contains("'s message"),
        "no sender context when key is malformed: {text}"
    );
}

#[test]
fn build_reaction_event_text_strips_transport_suffix_from_sender() {
    // Sender short-name = role part before `:` in "dev:claude-code".
    // The synthesized text names "dev" not "dev:claude-code" so
    // agents read the canonical role, not the transport identity.
    let body = reaction_toggle("ryan", "👍", "2026-04-23T11:30:00Z|designer:claude-code");
    let text = build_reaction_event_text_with_excerpt(&body, true, None);
    assert!(
        text.contains("designer") && !text.contains(":claude-code"),
        "sender should render as short-role: {text}"
    );
}

// ── #400 excerpt-enrichment tests ────────────────────────────────

#[test]
fn truncate_excerpt_returns_body_unchanged_when_short() {
    // Designer 12:08 UTC: skip the ellipsis when the full text fits.
    // Under max_len → no truncation + no trailing `…`.
    let out = truncate_excerpt("thanks!", 40);
    assert_eq!(out, "thanks!");
    assert!(!out.contains('…'));
}

#[test]
fn truncate_excerpt_cuts_on_word_boundary_and_appends_ellipsis() {
    // Long body truncates at the last whitespace ≤ max_len so the
    // excerpt ends on a complete word, not mid-word. The `…`
    // suffix signals truncation.
    let long = "this is a fairly long message that definitely exceeds forty chars for sure";
    let out = truncate_excerpt(long, 40);
    assert!(
        out.ends_with('…'),
        "truncated excerpt must carry the ellipsis marker: {out}"
    );
    // Assert the output doesn't end mid-word: the char before the
    // `…` is either a full word's last char or end-of-content.
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
        session::Message {
            from: "dev:claude-code".to_string(),
            text: "merged #377".to_string(),
            ts,
            ..session::Message::default()
        },
        session::Message {
            from: "pm:claude-code".to_string(),
            text: "nice".to_string(),
            ts: ts + chrono::Duration::seconds(1),
            ..session::Message::default()
        },
    ];
    let key = format!("{}|dev:claude-code", ts.to_rfc3339());
    let excerpt = find_message_excerpt(&msgs, &key, 40);
    assert_eq!(excerpt.as_deref(), Some("merged #377"));
}

#[test]
fn find_message_excerpt_returns_none_on_lookup_failure() {
    // Empty message set + malformed keys + non-matching ts all
    // return None — the broadcast path falls back to the no-excerpt
    // shape (designer 12:02 UTC: fallback preserves working path).
    assert!(find_message_excerpt(&[], "2026-04-23T00:00:00Z|dev", 40).is_none());
    assert!(find_message_excerpt(&[], "malformed-key", 40).is_none());
    assert!(find_message_excerpt(&[], "|", 40).is_none());
}

#[test]
fn build_reaction_event_text_with_excerpt_wraps_in_typographic_quotes() {
    // Excerpt wrapped in `"…"` (U+201C / U+201D) avoids collision
    // with any `"` chars inside the body text — the falsifier (c)
    // quote-safety case.
    let body = reaction_toggle("ryan", "👍", "2026-04-23T11:30:00Z|dev:claude-code");
    let text = build_reaction_event_text_with_excerpt(&body, true, Some("merged #377"));
    assert!(
        text.contains("\u{201C}merged #377\u{201D}"),
        "excerpt must wrap in typographic quotes: {text}"
    );
    assert!(text.contains("dev") && text.contains("👍"));
}

#[test]
fn build_reaction_event_text_with_excerpt_falls_back_when_excerpt_none() {
    // None passthrough keeps the #387 pre-excerpt shape — no
    // typographic quote, just the base synthesis. Guards the
    // falsifier (b) "lookup failure doesn't break working path."
    let body = reaction_toggle("ryan", "👍", "2026-04-23T11:30:00Z|dev:claude-code");
    let with_none = build_reaction_event_text_with_excerpt(&body, true, None);
    assert_eq!(with_none, "ryan reacted 👍 to dev's message");
    assert!(!with_none.contains('\u{201C}'));
}

#[tokio::test]
async fn post_reaction_cascade_threads_excerpt_into_broadcast_event_text() {
    use chrono::TimeZone;
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx.clone(),
    ));

    let target_ts = chrono::Utc
        .with_ymd_and_hms(2026, 4, 23, 11, 30, 0)
        .unwrap();
    let target_msg = session::Message {
        from: "dev:claude-code".to_string(),
        text: "merged #377 into main with the auth-cookie fix".to_string(),
        ts: target_ts,
        ..session::Message::default()
    };
    session.broadcast(&target_msg).expect("seed target message");

    let router = axum::Router::new()
        .route("/api/reactions", axum::routing::post(post_reaction))
        .with_state(session.clone());

    let message_key = format!("{}|dev:claude-code", target_ts.to_rfc3339());
    let toggle_body = format!(
        r#"{{"channel":"broadcast","message_key":"{message_key}","emoji":"👍","user":"ryan"}}"#
    );
    let request = Request::builder()
        .method("POST")
        .uri("/api/reactions")
        .header("Content-Type", "application/json")
        .body(Body::from(toggle_body))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let messages = session.read_messages().expect("read messages");
    let reaction_event = messages
        .iter()
        .find(|m| {
            m.from == "bpmux"
                && m.message_type == MessageType::Insight
                && m.text.contains("reacted")
        })
        .expect("synthesized reaction event must broadcast to session log");

    assert!(
        reaction_event
            .text
            .contains("ryan reacted 👍 to dev's message"),
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
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx.clone(),
    ));

    let router = axum::Router::new()
        .route("/api/reactions", axum::routing::post(post_reaction))
        .with_state(session.clone());

    let toggle_body = r#"{"channel":"broadcast","message_key":"2026-04-23T11:30:00+00:00|dev:claude-code","emoji":"👍","user":"ryan"}"#;
    let request = Request::builder()
        .method("POST")
        .uri("/api/reactions")
        .header("Content-Type", "application/json")
        .body(Body::from(toggle_body))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let messages = session.read_messages().expect("read messages");
    let reaction_event = messages
        .iter()
        .find(|m| {
            m.from == "bpmux"
                && m.message_type == MessageType::Insight
                && m.text.contains("reacted")
        })
        .expect("reaction event broadcasts even when lookup fails");

    assert_eq!(
        reaction_event.text, "ryan reacted 👍 to dev's message",
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
    use crate::transport::ci_state_store::CiStateStore;
    let store = CiStateStore::new();
    let router = axum::Router::new()
        .route("/api/ci", axum::routing::get(super::rest::get_ci))
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
    use crate::bridge::tools::CiStateReport;
    use crate::transport::ci_state_store::CiStateStore;
    let store = CiStateStore::new();
    let report = CiStateReport {
        conclusion: "success",
        last_run_id: Some(42),
        branch: "main",
        url: "https://github.com/RyanDur/bpmux/actions".to_string(),
        head_sha: Some("abc1234".to_string()),
        display_title: Some("test".to_string()),
        last_updated_at: None,
        last_successful_poll_at: None,
        poll_interval_secs: 60,
        backoff_cap_secs: 3600,
    };
    store.write(report);
    let router = axum::Router::new()
        .route("/api/ci", axum::routing::get(super::rest::get_ci))
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
    use crate::bridge::tools::CiStateReport;
    use crate::transport::ci_state_store::CiStateStore;
    let store = CiStateStore::new();
    store.write(CiStateReport {
        conclusion: "failure",
        last_run_id: Some(7),
        branch: "main",
        url: "https://github.com/RyanDur/bpmux/actions".to_string(),
        head_sha: Some("def5678".to_string()),
        display_title: Some("flaky test".to_string()),
        last_updated_at: None,
        last_successful_poll_at: None,
        poll_interval_secs: 60,
        backoff_cap_secs: 3600,
    });
    let router = axum::Router::new()
        .route("/api/ci", axum::routing::get(super::rest::get_ci))
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
