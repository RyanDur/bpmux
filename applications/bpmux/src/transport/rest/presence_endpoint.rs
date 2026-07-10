#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::Json;
use axum::extract::Extension;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;

use super::presence::*;

fn orchestrator() -> Arc<crate::store::Store> {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let (tx, _rx) = tokio::sync::broadcast::channel::<message::Message>(64);
    let hub = Arc::new(crate::hub::Hub::new(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
    ));
    std::mem::forget(dir);
    Arc::new(crate::store::Store::new(
        hub,
        None,
        crate::ci::CiStateStore::new(),
        tokio_util::sync::CancellationToken::new(),
    ))
}

fn select_with_dev_session() -> crate::store::Select {
    let seeded = crate::store::SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    seeded.dispatch(
        crate::store::routing::RoutingAction::Registered {
            user_id: "sess-dev".to_string(),
            display: "dev".to_string(),
            role: None,
        }
        .into(),
    );
    let select = seeded.select();
    std::mem::forget(seeded);
    select
}

fn dev_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("X-BPMUX-Agent", HeaderValue::from_static("dev"));
    headers
}

async fn post_state(state: &str) -> StatusCode {
    let response = post_presence(
        dev_headers(),
        Extension(select_with_dev_session()),
        Extension(orchestrator()),
        Json(PresenceRequest {
            state: state.to_string(),
        }),
    )
    .await
    .into_response();
    response.status()
}

#[tokio::test]
async fn the_endpoint_accepts_the_three_agent_pushable_states() {
    for state in ["busy", "idle", "help"] {
        assert_eq!(
            post_state(state).await,
            StatusCode::OK,
            "an agent pushing '{state}' through POST /api/presence sets its OWN roster presence — busy/idle/help are the agent-driven colour-source the dot reads"
        );
    }
}

#[tokio::test]
async fn the_endpoint_normalizes_case_and_whitespace() {
    assert_eq!(
        post_state("  HELP ").await,
        StatusCode::OK,
        "the verb normalizes like agent_from_headers does for the role — '  HELP ' is the same agent-pushable help"
    );
}

#[tokio::test]
async fn the_endpoint_rejects_states_an_agent_must_not_self_assign() {
    for forbidden in ["online", "offline", "blocked", "nonsense"] {
        assert_eq!(
            post_state(forbidden).await,
            StatusCode::BAD_REQUEST,
            "'{forbidden}' is NOT agent-pushable: online/offline are connection-derived (join/disconnect), blocked is the owner-park's to set, and nonsense is no presence at all — the endpoint rejects each with 400, never silently accepting"
        );
    }
}
