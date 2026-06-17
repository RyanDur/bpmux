use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

use super::init_session;
use crate::session;
use crate::transport::rest::wait_for_event;

fn wait_app(dir: &TempDir) -> (Arc<session::Session>, axum::Router) {
    let (tx, _rx) = broadcast::channel(16);
    let session = Arc::new(session::Session::new(
        dir.path().to_path_buf(),
        "dev".to_string(),
        tx,
    ));
    let routing = Arc::new(crate::session::routing_identity::RoutingStore::load(
        dir.path().to_path_buf(),
    ));
    let router = axum::Router::new()
        .route("/api/wait-for-event", axum::routing::get(wait_for_event))
        .layer(axum::Extension(routing))
        .with_state(session.clone());
    (session, router)
}

async fn body_json(response: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).expect("parse json")
}

fn broadcast_chat(from: &str, text: &str) -> session::Message {
    session::Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::Chat,
        ..session::Message::default()
    }
}

fn directed_chat(from: &str, text: &str, to: &[&str]) -> session::Message {
    session::Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::Chat,
        to: Some(to.iter().map(|r| (*r).to_string()).collect()),
        explicit_privacy: true,
        ..session::Message::default()
    }
}

#[tokio::test]
async fn returns_a_pending_broadcast_after_the_agent_subscribes() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, router) = wait_app(&dir);
    session
        .broadcast(&broadcast_chat("pm", "hello team"))
        .expect("broadcast");
    let request = Request::builder()
        .uri("/api/wait-for-event?as=dev&timeout_ms=2000")
        .body(Body::empty())
        .expect("request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let event = body_json(response).await;
    assert_eq!(event["event_class"], "broadcast_received");
    assert_eq!(event["payload"]["from"], "pm");
    assert_eq!(event["payload"]["text"], "hello team");
    assert_eq!(event["payload"]["channel"], "broadcast");
    assert!(event["cursor"].is_string());
}

#[tokio::test]
async fn blocks_then_returns_a_broadcast_dispatched_while_waiting() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, router) = wait_app(&dir);
    let waiter = tokio::spawn(async move {
        let request = Request::builder()
            .uri("/api/wait-for-event?as=dev&timeout_ms=3000")
            .body(Body::empty())
            .expect("request");
        router.oneshot(request).await.expect("response")
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    session
        .broadcast(&broadcast_chat("pm", "live message"))
        .expect("broadcast");
    let response = waiter.await.expect("join");
    assert_eq!(response.status(), StatusCode::OK);
    let event = body_json(response).await;
    assert_eq!(event["payload"]["text"], "live message");
}

#[tokio::test]
async fn times_out_cleanly_when_no_event_is_pending() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (_session, router) = wait_app(&dir);
    let request = Request::builder()
        .uri("/api/wait-for-event?as=dev&timeout_ms=200")
        .body(Body::empty())
        .expect("request");
    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let event = body_json(response).await;
    assert_eq!(event["timed_out"], true);
}

#[tokio::test]
async fn advancing_the_cursor_does_not_redeliver_the_same_event() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, router) = wait_app(&dir);
    session
        .broadcast(&broadcast_chat("pm", "first"))
        .expect("broadcast");
    let first_request = Request::builder()
        .uri("/api/wait-for-event?as=dev&timeout_ms=2000")
        .body(Body::empty())
        .expect("request");
    let first = body_json(
        router
            .clone()
            .oneshot(first_request)
            .await
            .expect("response"),
    )
    .await;
    let cursor = first["cursor"].as_str().expect("cursor").to_string();
    let second_request = Request::builder()
        .uri(format!(
            "/api/wait-for-event?as=dev&after={cursor}&timeout_ms=200"
        ))
        .body(Body::empty())
        .expect("request");
    let second = body_json(router.oneshot(second_request).await.expect("response")).await;
    assert_eq!(second["timed_out"], true);
}

#[tokio::test]
async fn a_message_directed_to_another_role_is_not_delivered() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, router) = wait_app(&dir);
    session
        .broadcast(&directed_chat("pm", "for designer only", &["designer"]))
        .expect("broadcast");
    let request = Request::builder()
        .uri("/api/wait-for-event?as=dev&timeout_ms=200")
        .body(Body::empty())
        .expect("request");
    let event = body_json(router.oneshot(request).await.expect("response")).await;
    assert_eq!(event["timed_out"], true);
}

#[tokio::test]
async fn an_agents_own_broadcast_is_not_delivered_back_to_it() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, router) = wait_app(&dir);
    session
        .broadcast(&broadcast_chat("dev", "my own message"))
        .expect("broadcast");
    let request = Request::builder()
        .uri("/api/wait-for-event?as=dev&timeout_ms=200")
        .body(Body::empty())
        .expect("request");
    let event = body_json(router.oneshot(request).await.expect("response")).await;
    assert_eq!(event["timed_out"], true);
}

fn state_delta_message(delta: session::message::StateDeltaPayload) -> session::Message {
    session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(delta),
        ..session::Message::default()
    }
}

#[tokio::test]
async fn delivers_a_story_transition_state_delta_as_a_subscription_event() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, router) = wait_app(&dir);
    session
        .broadcast(&state_delta_message(
            session::message::StateDeltaPayload::StoryTransition {
                transition: session::message::StoryTransitionWire {
                    story_id: 42,
                    from_state: "ready".to_string(),
                    target_state: "in-progress".to_string(),
                    actor: "dev".to_string(),
                },
            },
        ))
        .expect("broadcast");
    let request = Request::builder()
        .uri("/api/wait-for-event?as=designer&timeout_ms=2000")
        .body(Body::empty())
        .expect("request");
    let event = body_json(router.oneshot(request).await.expect("response")).await;
    assert_eq!(event["event_class"], "story_state_transition");
    assert_eq!(event["payload"]["story_id"], 42);
    assert_eq!(event["payload"]["target_state"], "in-progress");
}

#[tokio::test]
async fn a_gui_only_delta_is_not_delivered_as_an_agent_event() {
    let dir = TempDir::new().expect("tmpdir");
    init_session(&dir);
    let (session, router) = wait_app(&dir);
    session
        .broadcast(&state_delta_message(
            session::message::StateDeltaPayload::Roster { members: vec![] },
        ))
        .expect("broadcast");
    let request = Request::builder()
        .uri("/api/wait-for-event?as=designer&timeout_ms=200")
        .body(Body::empty())
        .expect("request");
    let event = body_json(router.oneshot(request).await.expect("response")).await;
    assert_eq!(event["timed_out"], true);
}
