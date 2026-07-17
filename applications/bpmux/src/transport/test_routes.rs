use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use serde::Deserialize;

use crate::bridge;
use crate::hub;
use crate::sync::OrchestratorHandle;
use crate::sync::RefreshDriver;
use message::{Message, MessageType};

fn fire_board_invalidate(hub: &Arc<hub::Hub>) {
    let delta = Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: Utc::now(),
        message_type: MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::BoardInvalidated {
            reason: "board changed".to_string(),
        }),
        ..Default::default()
    };
    if let Err(e) = hub.broadcast(&delta) {
        diag::log_diag(&format!(
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
    hub: Arc<hub::Hub>,
    refresh: OrchestratorHandle,
    select: crate::store::Select,
    dispatch: F,
) -> (StatusCode, Json<serde_json::Value>)
where
    F: FnOnce(Arc<dyn RefreshDriver>, Arc<hub::Hub>) -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let Some(refresh) = refresh else {
        return test_route_unavailable();
    };
    match dispatch(refresh.clone(), hub.clone()).await {
        Ok(message) => {
            fire_board_invalidate(&hub);
            crate::transport::rest::shared::broadcast_stories(&hub, &select);
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
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(dispatcher): axum::Extension<crate::store::Dispatch>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    Json(req): Json<CreateStoryReq>,
) -> impl IntoResponse {
    let select_for_dispatch = select.clone();
    run_test_route(hub, refresh, select, |refresh, _hub| async move {
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
        let role = bridge::tools_stories::role_from_name(&agent);
        bridge::dispatch_create_story(
            args,
            &agent,
            role,
            None,
            Some(&refresh),
            &select_for_dispatch,
            &dispatcher,
        )
        .await
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct CreateIterationReq {
    pub name: String,
    pub agent: Option<String>,
}

pub async fn post_test_create_iteration(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(dispatcher): axum::Extension<crate::store::Dispatch>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    Json(req): Json<CreateIterationReq>,
) -> impl IntoResponse {
    let create_select = select.clone();
    run_test_route(hub, refresh, select, |refresh, _hub| async move {
        let mut args = serde_json::Map::new();
        put_str(&mut args, "name", req.name);
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        let role = bridge::tools_stories::role_from_name(&agent);
        bridge::dispatch_create_iteration(
            args,
            &agent,
            role,
            None,
            Some(&refresh),
            &dispatcher,
            &create_select,
        )
        .await
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct DesignateCurrentIterationReq {
    pub iteration_id: u64,
    pub agent: Option<String>,
}

pub async fn post_test_designate_current_iteration(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(dispatcher): axum::Extension<crate::store::Dispatch>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    Json(req): Json<DesignateCurrentIterationReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return test_route_unavailable();
    };
    let mut args = serde_json::Map::new();
    args.insert(
        "iteration_id".to_string(),
        serde_json::Value::Number(req.iteration_id.into()),
    );
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let role = bridge::tools_stories::role_from_name(&agent);
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_designate_current_iteration(
        args,
        &agent,
        role,
        None,
        Some(refresh_ref),
        &dispatcher,
        &select,
    )
    .await
    {
        Ok(message) => {
            if let Err(e) = refresh.drain_pending().await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": format!("drain_pending after designate failed: {e}")
                    })),
                );
            }
            fire_board_invalidate(&hub);
            crate::transport::rest::shared::broadcast_stories(&hub, &select);
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
    pub id: u64,
    pub target_state: String,
    pub rationale: String,
    pub position: Option<u64>,
    pub agent: Option<String>,
}

pub async fn post_test_move_story(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(dispatcher): axum::Extension<crate::store::Dispatch>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    Json(req): Json<MoveStoryReq>,
) -> impl IntoResponse {
    let select_for_dispatch = select.clone();
    run_test_route(hub, refresh, select, |refresh, _hub| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "id", req.id);
        put_str(&mut args, "target_state", req.target_state);
        put_str(&mut args, "rationale", req.rationale);
        if let Some(p) = req.position {
            put_num(&mut args, "position", p);
        }
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        let role = bridge::tools_stories::role_from_name(&agent);
        bridge::dispatch_move_story(
            args,
            &agent,
            role,
            None,
            Some(&refresh),
            &dispatcher,
            &select_for_dispatch,
        )
        .await
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct EditStoryReq {
    pub id: u64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub agent: Option<String>,
}

pub async fn post_test_edit_story(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(dispatcher): axum::Extension<crate::store::Dispatch>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    Json(req): Json<EditStoryReq>,
) -> impl IntoResponse {
    let select_for_dispatch = select.clone();
    run_test_route(hub, refresh, select, |refresh, _hub| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "id", req.id);
        if let Some(t) = req.title {
            put_str(&mut args, "title", t);
        }
        if let Some(b) = req.body {
            put_str(&mut args, "body", b);
        }
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        let role = bridge::tools_stories::role_from_name(&agent);
        bridge::dispatch_edit_story(
            args,
            &agent,
            role,
            None,
            Some(&refresh),
            &select_for_dispatch,
            &dispatcher,
        )
        .await
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct CommentOnStoryReq {
    pub id: u64,
    pub body: String,
    pub agent: Option<String>,
}

pub async fn post_test_comment_on_story(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    Json(req): Json<CommentOnStoryReq>,
) -> impl IntoResponse {
    let select_for_dispatch = select.clone();
    run_test_route(hub, refresh, select, |refresh, _hub| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "id", req.id);
        put_str(&mut args, "body", req.body);
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        bridge::dispatch_comment_on_story(args, &agent, None, Some(&refresh), &select_for_dispatch)
            .await
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct StartStoryReq {
    pub id: u64,
    pub pickup_paraphrase: String,
    pub falsifier_reframing: String,
    pub agent: Option<String>,
}

pub async fn post_test_start_story(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    Json(req): Json<StartStoryReq>,
) -> impl IntoResponse {
    let select_for_dispatch = select.clone();
    run_test_route(hub, refresh, select, |refresh, hub| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "id", req.id);
        put_str(&mut args, "pickup_paraphrase", req.pickup_paraphrase);
        put_str(&mut args, "falsifier_reframing", req.falsifier_reframing);
        let agent = req.agent.unwrap_or_else(|| "dev:test".to_string());
        let role = bridge::tools_stories::role_from_name(&agent);
        bridge::dispatch_start_story(
            args,
            &agent,
            role,
            None,
            Some(&refresh),
            &select_for_dispatch,
            &hub,
        )
        .await
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct FailDeskCheckReq {
    pub id: u64,
    pub reason: String,
    pub agent: Option<String>,
}

pub async fn post_test_fail_desk_check(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    Json(req): Json<FailDeskCheckReq>,
) -> impl IntoResponse {
    let select_for_dispatch = select.clone();
    run_test_route(hub, refresh, select, |refresh, hub| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "id", req.id);
        put_str(&mut args, "reason", req.reason);
        let agent = req.agent.unwrap_or_else(|| "designer:test".to_string());
        let role = bridge::tools_stories::role_from_name(&agent);
        bridge::dispatch_fail_desk_check(
            args,
            &agent,
            role,
            None,
            Some(&refresh),
            &select_for_dispatch,
            &hub,
        )
        .await
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct AcceptStoryReq {
    pub id: u64,
    pub agent: Option<String>,
}

pub async fn post_test_accept_story(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    axum::Extension(race_window): axum::Extension<crate::sync::RaceWindowHandle>,
    Json(req): Json<AcceptStoryReq>,
) -> impl IntoResponse {
    let select_for_dispatch = select.clone();
    run_test_route(hub, refresh, select, |refresh, hub| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "id", req.id);
        let agent = req.agent.unwrap_or_else(|| "owner:test".to_string());
        let role = bridge::tools_stories::role_from_name(&agent);
        bridge::dispatch_accept_story(
            args,
            &agent,
            role,
            None,
            Some(&refresh),
            &select_for_dispatch,
            &hub,
            &race_window,
        )
        .await
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct DeliverStoryReq {
    pub id: u64,
    pub verified_observation: String,
    pub agent: Option<String>,
}

pub async fn post_test_deliver_story(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    Json(req): Json<DeliverStoryReq>,
) -> impl IntoResponse {
    let select_for_dispatch = select.clone();
    run_test_route(hub, refresh, select, |refresh, hub| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "id", req.id);
        put_str(&mut args, "verified_observation", req.verified_observation);
        let agent = req.agent.unwrap_or_else(|| "dev:test".to_string());
        let role = bridge::tools_stories::role_from_name(&agent);
        bridge::dispatch_deliver_story(
            args,
            &agent,
            role,
            None,
            Some(&refresh),
            &select_for_dispatch,
            &hub,
        )
        .await
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct CloseStoryReq {
    pub id: u64,
    pub reason: String,
    pub agent: Option<String>,
    pub disposition: Option<String>,
}

pub async fn post_test_close_story(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    Json(req): Json<CloseStoryReq>,
) -> impl IntoResponse {
    let select_for_dispatch = select.clone();
    run_test_route(hub, refresh, select, |refresh, _hub| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "id", req.id);
        put_str(&mut args, "reason", req.reason);
        if let Some(disposition) = req.disposition {
            put_str(&mut args, "disposition", disposition);
        }
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        let role = bridge::tools_stories::role_from_name(&agent);
        bridge::dispatch_close_story(
            args,
            &agent,
            role,
            None,
            Some(&refresh),
            &select_for_dispatch,
        )
        .await
    })
    .await
}

#[derive(Debug, Deserialize)]
pub struct CloseIterationReq {
    pub iteration_id: u64,
    pub agent: Option<String>,
}

pub async fn post_test_close_iteration(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(dispatcher): axum::Extension<crate::store::Dispatch>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    Json(req): Json<CloseIterationReq>,
) -> impl IntoResponse {
    let close_select = select.clone();
    run_test_route(hub, refresh, select, |refresh, hub| async move {
        let mut args = serde_json::Map::new();
        put_num(&mut args, "iteration_id", req.iteration_id);
        let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
        let role = bridge::tools_stories::role_from_name(&agent);
        bridge::dispatch_close_iteration(
            args,
            role,
            Some(&refresh),
            &hub,
            &dispatcher,
            &close_select,
        )
        .await
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
    axum::Extension(store): axum::Extension<Arc<crate::store::Store>>,
    Json(req): Json<FireCiStateReq>,
) -> impl IntoResponse {
    let conclusion: &'static str = match req.conclusion.as_str() {
        "success" => "success",
        "failure" => "failure",
        "in_progress" => "in_progress",
        _ => "unknown",
    };
    let snapshot = crate::store::CiSnapshot {
        state: ci_state_for_conclusion(conclusion),
        run_id: Some(req.run_id),
        head_sha: Some(req.head_sha),
        display_title: Some("test ci transition".to_string()),
        conclusion,
        workflows: Vec::new(),
    };
    store
        .dispatch(crate::store::AppAction::Ci(
            crate::store::CiAction::Updated(snapshot),
        ))
        .await;
    (
        StatusCode::OK,
        Json(serde_json::json!({ "dispatched": true })),
    )
}
