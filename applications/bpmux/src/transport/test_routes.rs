use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use serde::Deserialize;

use crate::board::RefreshDriver;
use crate::bridge;
use crate::server::TestRefreshHandle;
use crate::session::{self, Message, MessageType};

/// Fire BOARD_INVALIDATE_MARKER through the session's broadcast
/// channel so connected WS clients (the Electron renderer in
/// Playwright tests) refetch their board state without manual
/// reload (#490). Mirrors the production path where bridge
/// subprocess mutations broadcast the marker as a system-typed
/// chat message; test-routes don't go through the bridge subprocess
/// (axum-direct path uses an empty WsSink), so this restores the
/// live-update semantic for Playwright observation-class specs.
fn fire_board_invalidate(session: &Arc<session::Session>) {
    let msg = Message {
        from: "bpmux:test".to_string(),
        text: crate::bridge::BOARD_INVALIDATE_MARKER.to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Default::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        crate::session::diag::log_diag(&format!(
            "test-route: BOARD_INVALIDATE broadcast failed: {e}"
        ));
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateStoryReq {
    pub title: String,
    pub user_job: String,
    pub observation_falsifier: String,
    pub body: Option<String>,
    pub parent_epic: Option<u64>,
    pub agent: Option<String>,
}

pub async fn post_test_create_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<CreateStoryReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
            })),
        );
    };
    let mut args = serde_json::Map::new();
    args.insert("title".to_string(), serde_json::Value::String(req.title));
    args.insert(
        "user_job".to_string(),
        serde_json::Value::String(req.user_job),
    );
    args.insert(
        "observation_falsifier".to_string(),
        serde_json::Value::String(req.observation_falsifier),
    );
    if let Some(body) = req.body {
        args.insert("body".to_string(), serde_json::Value::String(body));
    }
    if let Some(epic) = req.parent_epic {
        args.insert(
            "parent_epic".to_string(),
            serde_json::Value::Number(epic.into()),
        );
    }
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_create_story(args, &agent, Some(refresh_ref)).await {
        Ok(message) => {
            fire_board_invalidate(&session);
            (
                StatusCode::OK,
                Json(serde_json::json!({ "message": message })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct CommentOnStoryReq {
    pub number: u64,
    pub body: String,
    pub agent: Option<String>,
}

pub async fn post_test_comment_on_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<CommentOnStoryReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
            })),
        );
    };
    let mut args = serde_json::Map::new();
    args.insert(
        "number".to_string(),
        serde_json::Value::Number(req.number.into()),
    );
    args.insert("body".to_string(), serde_json::Value::String(req.body));
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_comment_on_story(args, &agent, Some(refresh_ref)).await {
        Ok(message) => {
            fire_board_invalidate(&session);
            (
                StatusCode::OK,
                Json(serde_json::json!({ "message": message })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct StartStoryReq {
    pub number: u64,
    pub pickup_paraphrase: String,
    pub falsifier_reframing: String,
    pub agent: Option<String>,
}

pub async fn post_test_start_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<StartStoryReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
            })),
        );
    };
    let mut args = serde_json::Map::new();
    args.insert(
        "number".to_string(),
        serde_json::Value::Number(req.number.into()),
    );
    args.insert(
        "pickup_paraphrase".to_string(),
        serde_json::Value::String(req.pickup_paraphrase),
    );
    args.insert(
        "falsifier_reframing".to_string(),
        serde_json::Value::String(req.falsifier_reframing),
    );
    let agent = req.agent.unwrap_or_else(|| "dev:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_start_story(args, &agent, Some(refresh_ref)).await {
        Ok(message) => {
            fire_board_invalidate(&session);
            (
                StatusCode::OK,
                Json(serde_json::json!({ "message": message })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct FailDeskCheckReq {
    pub number: u64,
    pub reason: String,
    pub agent: Option<String>,
}

pub async fn post_test_fail_desk_check(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<FailDeskCheckReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
            })),
        );
    };
    let mut args = serde_json::Map::new();
    args.insert(
        "number".to_string(),
        serde_json::Value::Number(req.number.into()),
    );
    args.insert("reason".to_string(), serde_json::Value::String(req.reason));
    let agent = req.agent.unwrap_or_else(|| "designer:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_fail_desk_check(args, &agent, Some(refresh_ref)).await {
        Ok(message) => {
            fire_board_invalidate(&session);
            (
                StatusCode::OK,
                Json(serde_json::json!({ "message": message })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct AcceptStoryReq {
    pub number: u64,
    pub agent: Option<String>,
}

pub async fn post_test_accept_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<AcceptStoryReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
            })),
        );
    };
    let mut args = serde_json::Map::new();
    args.insert(
        "number".to_string(),
        serde_json::Value::Number(req.number.into()),
    );
    let agent = req.agent.unwrap_or_else(|| "ryan:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_accept_story(args, &agent, Some(refresh_ref)).await {
        Ok(message) => {
            fire_board_invalidate(&session);
            (
                StatusCode::OK,
                Json(serde_json::json!({ "message": message })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct DeliverStoryReq {
    pub number: u64,
    pub verified_observation: String,
    pub agent: Option<String>,
}

pub async fn post_test_deliver_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<DeliverStoryReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
            })),
        );
    };
    let mut args = serde_json::Map::new();
    args.insert(
        "number".to_string(),
        serde_json::Value::Number(req.number.into()),
    );
    args.insert(
        "verified_observation".to_string(),
        serde_json::Value::String(req.verified_observation),
    );
    let agent = req.agent.unwrap_or_else(|| "dev:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_deliver_story(args, &agent, Some(refresh_ref)).await {
        Ok(message) => {
            fire_board_invalidate(&session);
            (
                StatusCode::OK,
                Json(serde_json::json!({ "message": message })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct CloseStoryReq {
    pub number: u64,
    pub reason: String,
    pub agent: Option<String>,
}

pub async fn post_test_close_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<CloseStoryReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
            })),
        );
    };
    let mut args = serde_json::Map::new();
    args.insert(
        "number".to_string(),
        serde_json::Value::Number(req.number.into()),
    );
    args.insert("reason".to_string(), serde_json::Value::String(req.reason));
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_close_story(args, &agent, Some(refresh_ref)).await {
        Ok(message) => {
            fire_board_invalidate(&session);
            (
                StatusCode::OK,
                Json(serde_json::json!({ "message": message })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}
