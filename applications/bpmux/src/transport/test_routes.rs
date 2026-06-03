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
    let delta = Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: Utc::now(),
        message_type: MessageType::StateDelta,
        state_delta: Some(
            crate::session::message::StateDeltaPayload::BoardInvalidated {
                reason: "board changed".to_string(),
            },
        ),
        ..Default::default()
    };
    if let Err(e) = session.broadcast(&delta) {
        crate::session::diag::log_diag(&format!(
            "test-route: board_invalidated state-delta broadcast failed: {e}"
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
    pub acceptance_test_path: Option<String>,
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
    let path = req
        .acceptance_test_path
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml").to_string());
    args.insert(
        "acceptance_test_path".to_string(),
        serde_json::Value::String(path),
    );
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
pub struct CreateIterationReq {
    pub theme_name: String,
    pub agent: Option<String>,
}

pub async fn post_test_create_iteration(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<CreateIterationReq>,
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
        "theme_name".to_string(),
        serde_json::Value::String(req.theme_name),
    );
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_create_iteration(args, &agent, Some(refresh_ref)).await {
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
pub struct DesignateCurrentIterationReq {
    pub iteration_number: u64,
    pub agent: Option<String>,
}

pub async fn post_test_designate_current_iteration(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<DesignateCurrentIterationReq>,
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
        "iteration_number".to_string(),
        serde_json::Value::Number(req.iteration_number.into()),
    );
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_designate_current_iteration(args, &agent, Some(refresh_ref)).await {
        Ok(message) => {
            if let Err(e) = refresh.drain_pending().await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": format!("drain_pending after designate failed: {e}")
                    })),
                );
            }
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
pub struct MoveStoryReq {
    pub number: u64,
    pub target_state: String,
    pub rationale: String,
    pub position: Option<u64>,
    pub agent: Option<String>,
}

pub async fn post_test_move_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<MoveStoryReq>,
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
        "target_state".to_string(),
        serde_json::Value::String(req.target_state),
    );
    args.insert(
        "rationale".to_string(),
        serde_json::Value::String(req.rationale),
    );
    if let Some(p) = req.position {
        args.insert("position".to_string(), serde_json::Value::Number(p.into()));
    }
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_move_story(args, &agent, Some(refresh_ref)).await {
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
pub struct EditStoryReq {
    pub number: u64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub agent: Option<String>,
}

pub async fn post_test_edit_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<EditStoryReq>,
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
    if let Some(t) = req.title {
        args.insert("title".to_string(), serde_json::Value::String(t));
    }
    if let Some(b) = req.body {
        args.insert("body".to_string(), serde_json::Value::String(b));
    }
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_edit_story(args, &agent, Some(refresh_ref)).await {
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
    match bridge::dispatch_test_start_story(args, &agent, Some(refresh_ref), &session).await {
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
    match bridge::dispatch_test_fail_desk_check(args, &agent, Some(refresh_ref), &session).await {
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
    match bridge::dispatch_test_accept_story(args, &agent, Some(refresh_ref), &session).await {
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
    match bridge::dispatch_test_deliver_story(args, &agent, Some(refresh_ref), &session).await {
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

#[derive(Debug, Deserialize)]
pub struct FireCiStateReq {
    pub run_id: u64,
    pub head_sha: String,
    pub conclusion: String,
}

fn ci_state_for_conclusion(conclusion: &str) -> crate::ci::CiState {
    match conclusion {
        "success" => crate::ci::CiState::Green,
        "failure" => crate::ci::CiState::Red,
        "in_progress" => crate::ci::CiState::Running,
        _ => crate::ci::CiState::Unknown,
    }
}

pub async fn post_test_fire_ci_state(
    axum::Extension(orchestrator): axum::Extension<
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    >,
    Json(req): Json<FireCiStateReq>,
) -> impl IntoResponse {
    let conclusion: &'static str = match req.conclusion.as_str() {
        "success" => "success",
        "failure" => "failure",
        "in_progress" => "in_progress",
        _ => "unknown",
    };
    let snapshot = crate::orchestrator::CiSnapshot {
        state: ci_state_for_conclusion(conclusion),
        run_id: Some(req.run_id),
        head_sha: Some(req.head_sha),
        display_title: Some("test ci transition".to_string()),
        conclusion,
        workflows: Vec::new(),
    };
    orchestrator
        .dispatch(crate::orchestrator::OrchestratorAction::CiUpdated(snapshot))
        .await;
    (
        StatusCode::OK,
        Json(serde_json::json!({ "dispatched": true })),
    )
}

#[derive(Debug, Deserialize)]
pub struct ViewVelocityReq {
    pub messages: Vec<Message>,
}

pub async fn post_test_view_velocity(
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<ViewVelocityReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
            })),
        );
    };
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_view_velocity(req.messages, Some(refresh_ref)).await {
        Ok(metrics_json) => {
            let parsed: serde_json::Value = serde_json::from_str(&metrics_json)
                .unwrap_or(serde_json::Value::String(metrics_json));
            (
                StatusCode::OK,
                Json(serde_json::json!({ "metrics": parsed })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}
