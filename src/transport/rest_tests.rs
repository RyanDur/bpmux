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
            message_type: MessageType::Chat,
            to: None,
            addressee: None,
            explicit_privacy: false,
            thread_id: None,
            multi_depth: None,
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
            to: None,
            addressee: None,
            explicit_privacy: false,
            thread_id: None,
            multi_depth: None,
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
    // The rejected message must NOT appear in the broadcast stream —
    // topic_subs would mutate on the receiver side otherwise.
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
// /api/simulate/key tests
//
// Behavior-framed names because each test drives the real axum router
// through the real handler and asserts on the TerminalInjectEvent the
// handler enqueues. The transport-neutral inject type isolates this
// suite from crossterm; the crossterm mapping is owned by
// `terminal::event::TerminalInjectEvent::into_crossterm` and covered
// by that module's unit tests.
// ─────────────────────────────────────────────────────────────────────

use crate::terminal::event::TerminalInjectEvent;

fn test_app_with_key_injector() -> (
    axum::Router,
    tokio::sync::mpsc::Receiver<TerminalInjectEvent>,
) {
    let (inject_tx, inject_rx) = tokio::sync::mpsc::channel::<TerminalInjectEvent>(16);
    let router = axum::Router::new()
        .route("/api/simulate/key", axum::routing::post(simulate_key))
        .layer(axum::Extension(inject_tx));
    (router, inject_rx)
}

async fn post_key_payload(router: axum::Router, body: &str) -> axum::http::Response<Body> {
    let request = Request::builder()
        .method("POST")
        .uri("/api/simulate/key")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    router.oneshot(request).await.expect("response")
}

fn unwrap_key(event: TerminalInjectEvent) -> (String, Vec<String>) {
    match event {
        TerminalInjectEvent::Key { key, modifiers } => (key, modifiers),
        other => panic!("expected Key inject, got {other:?}"),
    }
}

#[tokio::test]
async fn simulate_key_enqueues_plain_key_inject_when_no_modifiers_field_present() {
    // Backward-compat: payload without `modifiers` field must behave
    // identically to the no-modifier path. Guards default-empty
    // deserialization.
    let (router, mut rx) = test_app_with_key_injector();
    let response = post_key_payload(router, r#"{"key":"1"}"#).await;
    assert_eq!(response.status(), StatusCode::OK);
    let (key, modifiers) = unwrap_key(rx.recv().await.expect("receive event"));
    assert_eq!(key, "1");
    assert!(modifiers.is_empty());
}

#[tokio::test]
async fn simulate_key_enqueues_alt_modifier_when_modifiers_contains_alt() {
    let (router, mut rx) = test_app_with_key_injector();
    let response = post_key_payload(router, r#"{"key":"1","modifiers":["alt"]}"#).await;
    assert_eq!(response.status(), StatusCode::OK);
    let (key, modifiers) = unwrap_key(rx.recv().await.expect("receive event"));
    assert_eq!(key, "1");
    assert!(
        modifiers.iter().any(|m| m.eq_ignore_ascii_case("alt")),
        "expected alt in modifiers, got {modifiers:?}"
    );
}

#[tokio::test]
async fn simulate_key_enqueues_multi_modifier_when_ctrl_and_shift_both_set() {
    let (router, mut rx) = test_app_with_key_injector();
    let response = post_key_payload(router, r#"{"key":"x","modifiers":["ctrl","shift"]}"#).await;
    assert_eq!(response.status(), StatusCode::OK);
    let (key, modifiers) = unwrap_key(rx.recv().await.expect("receive event"));
    assert_eq!(key, "x");
    assert!(modifiers.iter().any(|m| m.eq_ignore_ascii_case("ctrl")));
    assert!(modifiers.iter().any(|m| m.eq_ignore_ascii_case("shift")));
}

#[tokio::test]
async fn simulate_key_returns_bad_request_when_modifier_name_is_unknown() {
    // Unknown modifier strings must fail loudly — a silent ignore
    // leaves agents believing they exercised a modifier combo when
    // they exercised only the plain key.
    let (router, mut rx) = test_app_with_key_injector();
    let response = post_key_payload(router, r#"{"key":"1","modifiers":["meta"]}"#).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        rx.try_recv().is_err(),
        "unknown modifier must not enqueue event"
    );
}

#[tokio::test]
async fn simulate_key_passes_modifier_casing_through_unchanged() {
    // The transport layer preserves what the caller sent; case-
    // folding happens inside TerminalInjectEvent::into_crossterm.
    // Callers sending "ALT" or "Shift" still succeed.
    let (router, mut rx) = test_app_with_key_injector();
    let response = post_key_payload(router, r#"{"key":"a","modifiers":["ALT","Shift"]}"#).await;
    assert_eq!(response.status(), StatusCode::OK);
    let (key, modifiers) = unwrap_key(rx.recv().await.expect("receive event"));
    assert_eq!(key, "a");
    assert_eq!(modifiers, vec!["ALT".to_string(), "Shift".to_string()]);
}
