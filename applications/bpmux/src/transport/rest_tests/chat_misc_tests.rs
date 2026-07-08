#![allow(unused_imports)]

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use super::{init_session, seed_messages, seed_mixed_messages, test_app};
use crate::hub;
use crate::message::MessageType;
use crate::transport::rest::*;

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
    let _guard = bpmux_log_env_lock();
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (tx, _rx) = broadcast::channel(16);
    let hub = Arc::new(hub::Hub::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let router = axum::Router::new()
        .route(
            "/api/classifier-misfire",
            axum::routing::post(post_classifier_misfire),
        )
        .with_state(hub);

    let log_dir = TempDir::new().expect("log tmpdir");
    let log_path = log_dir.path().join("bpmux.log");
    let prev = std::env::var("BPMUX_LOG").ok();
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

    let forged_count = log_contents
        .lines()
        .filter(|l| l.contains("MISFIRE agent=\"fake\""))
        .count();
    assert_eq!(
        forged_count, 0,
        "adversarial newline + quotes must not forge a second MISFIRE line. log:\n{log_contents}"
    );
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
    let hub = Arc::new(hub::Hub::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let router = axum::Router::new()
        .route(
            "/api/classifier-misfire",
            axum::routing::post(post_classifier_misfire),
        )
        .with_state(hub);
    let request = Request::builder()
        .method("POST")
        .uri("/api/classifier-misfire")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"agent":"dev"}"#))
        .expect("build request");
    let response = router.oneshot(request).await.expect("response");
    assert_ne!(response.status(), StatusCode::CREATED);
}
