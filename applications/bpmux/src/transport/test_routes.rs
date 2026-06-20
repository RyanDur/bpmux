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

fn test_route_unavailable() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
        })),
    )
}

fn put_str(args: &mut serde_json::Map<String, serde_json::Value>, key: &str, val: String) {
    args.insert(key.to_string(), serde_json::Value::String(val));
}

fn put_num(args: &mut serde_json::Map<String, serde_json::Value>, key: &str, val: u64) {
    args.insert(key.to_string(), serde_json::Value::Number(val.into()));
}

async fn run_test_route<F, Fut>(
    session: Arc<session::Session>,
    refresh: TestRefreshHandle,
    dispatch: F,
) -> (StatusCode, Json<serde_json::Value>)
where
    F: FnOnce(Arc<dyn RefreshDriver>, Arc<session::Session>) -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let Some(refresh) = refresh else {
        return test_route_unavailable();
    };
    match dispatch(refresh.clone(), session.clone()).await {
        Ok(message) => {
            fire_board_invalidate(&session);
            crate::transport::rest::shared::broadcast_board_snapshot(&session, &refresh);
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
    run_test_route(session, refresh, |refresh, _session| async move {
        let mut args = serde_json::Map::new();
        put_str(&mut args, "title", req.title);
        put_str(&mut args, "user_job", req.user_job);
        put_str(
            &mut args,
            "observation_falsifier",
            req.observation_falsifier,
        );
        if let Some(body) = req.body {
            put_str(&mut args, "body", body);
        }
        if let Some(epic) = req.parent_epic {
            put_num(&mut args, "parent_epic", epic);
        }
        let path = req
            .acceptance_test_path
            .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml").to_string());
        put_str(&mut args, "acceptance_test_path", path);
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        bridge::dispatch_create_story(args, &agent, Some(&refresh)).await
    })
    .await
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
    run_test_route(session, refresh, |refresh, _session| async move {
        let mut args = serde_json::Map::new();
        put_str(&mut args, "theme_name", req.theme_name);
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        bridge::dispatch_create_iteration(args, &agent, Some(&refresh)).await
    })
    .await
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
        return test_route_unavailable();
    };
    let mut args = serde_json::Map::new();
    args.insert(
        "iteration_number".to_string(),
        serde_json::Value::Number(req.iteration_number.into()),
    );
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_designate_current_iteration(args, &agent, Some(refresh_ref)).await {
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
            crate::transport::rest::shared::broadcast_board_snapshot(&session, &refresh);
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
    run_test_route(session, refresh, |refresh, _session| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "number", req.number);
        put_str(&mut args, "target_state", req.target_state);
        put_str(&mut args, "rationale", req.rationale);
        if let Some(p) = req.position {
            put_num(&mut args, "position", p);
        }
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        bridge::dispatch_move_story(args, &agent, Some(&refresh)).await
    })
    .await
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
    run_test_route(session, refresh, |refresh, _session| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "number", req.number);
        if let Some(t) = req.title {
            put_str(&mut args, "title", t);
        }
        if let Some(b) = req.body {
            put_str(&mut args, "body", b);
        }
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        bridge::dispatch_edit_story(args, &agent, Some(&refresh)).await
    })
    .await
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
    run_test_route(session, refresh, |refresh, _session| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "number", req.number);
        put_str(&mut args, "body", req.body);
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        bridge::dispatch_comment_on_story(args, &agent, Some(&refresh)).await
    })
    .await
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
    run_test_route(session, refresh, |refresh, session| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "number", req.number);
        put_str(&mut args, "pickup_paraphrase", req.pickup_paraphrase);
        put_str(&mut args, "falsifier_reframing", req.falsifier_reframing);
        let agent = req.agent.unwrap_or_else(|| "dev:test".to_string());
        bridge::dispatch_start_story(args, &agent, Some(&refresh), &session).await
    })
    .await
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
    run_test_route(session, refresh, |refresh, session| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "number", req.number);
        put_str(&mut args, "reason", req.reason);
        let agent = req.agent.unwrap_or_else(|| "designer:test".to_string());
        bridge::dispatch_fail_desk_check(args, &agent, Some(&refresh), &session).await
    })
    .await
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
    run_test_route(session, refresh, |refresh, session| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "number", req.number);
        let agent = req.agent.unwrap_or_else(|| "owner:test".to_string());
        bridge::dispatch_accept_story(args, &agent, Some(&refresh), &session).await
    })
    .await
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
    run_test_route(session, refresh, |refresh, session| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "number", req.number);
        put_str(&mut args, "verified_observation", req.verified_observation);
        let agent = req.agent.unwrap_or_else(|| "dev:test".to_string());
        bridge::dispatch_deliver_story(args, &agent, Some(&refresh), &session).await
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct CloseStoryReq {
    pub number: u64,
    pub reason: String,
    pub agent: Option<String>,
    pub disposition: Option<String>,
}

pub async fn post_test_close_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<CloseStoryReq>,
) -> impl IntoResponse {
    run_test_route(session, refresh, |refresh, _session| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "number", req.number);
        put_str(&mut args, "reason", req.reason);
        if let Some(disposition) = req.disposition {
            put_str(&mut args, "disposition", disposition);
        }
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        bridge::dispatch_close_story(args, &agent, Some(&refresh)).await
    })
    .await
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
pub struct FireSyncStatusReq {
    pub status: String,
    pub count: Option<usize>,
}

pub async fn post_test_fire_sync_status(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<FireSyncStatusReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return test_route_unavailable();
    };
    let count = req.count.unwrap_or(1).max(1);
    let handle = refresh.status_handle();
    {
        let mut status = handle.lock().unwrap_or_else(|e| e.into_inner());
        *status = crate::board::SyncStatus::default();
        match req.status.as_str() {
            "conflict" => status.conflicts_pending = count,
            "stale" => status.stale = true,
            "drain" => status.pending_events = count,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("unknown sync status '{}'", req.status)
                    })),
                );
            }
        }
    }
    let snapshot = {
        let status = handle.lock().unwrap_or_else(|e| e.into_inner());
        status.clone()
    };
    crate::bridge::polling::broadcast_sync_status_heartbeat(&session, &snapshot);
    crate::bridge::polling::broadcast_sync_status_transition(&session, &snapshot);
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
        return test_route_unavailable();
    };
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_view_velocity(req.messages, Some(refresh_ref)).await {
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
