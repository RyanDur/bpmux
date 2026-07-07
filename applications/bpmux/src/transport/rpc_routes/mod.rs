use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde_json::Value;

use crate::session;
use crate::sync::OrchestratorHandle;

mod state_delta;
mod tool_dispatch;
mod validation;

#[cfg(test)]
mod rpc_dispatch_tests;

use state_delta::{
    fire_board_invalidate, fire_stories, fire_story_transition, fire_view_backlog_marker,
    story_state_label,
};
use tool_dispatch::dispatch;
pub use validation::{AGENT_HEADER, agent_from_headers};
use validation::{args_from_value, is_read_only_tool};

pub async fn post_rpc_tool(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(dispatcher): axum::Extension<crate::store::Dispatch>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    headers: HeaderMap,
    Path(tool): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let agent = match agent_from_headers(&headers) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            );
        }
    };
    let agent = crate::workflow::current_workflow()
        .role_id_for_purpose(&agent)
        .map(str::to_string)
        .unwrap_or(agent);
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "canonical store not configured (no GitHub orchestrator built at startup)"
            })),
        );
    };
    let args = match args_from_value(body) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            );
        }
    };
    let transition_id = args.get("id").and_then(|v| v.as_u64());
    let declared_target_state = if tool == "move_story" {
        args.get("target_state")
            .and_then(|v| v.as_str())
            .and_then(|raw| {
                use crate::product_management::scheduling::ProjectedState;
                match raw.to_ascii_lowercase().as_str() {
                    "icebox" => Some(ProjectedState::Icebox.as_str()),
                    "ready" => Some(ProjectedState::Ready.as_str()),
                    other => crate::product_management::story::Work::parse(other)
                        .filter(|s| *s != crate::product_management::story::Work::NotStarted)
                        .map(|s| s.as_str()),
                }
            })
    } else {
        None
    };
    let prior_state = transition_id
        .filter(|_| !is_read_only_tool(&tool))
        .and_then(|id| story_state_label(&refresh, id));
    match dispatch(
        &refresh,
        &tool,
        &agent,
        args,
        &session,
        &dispatcher,
        &select,
    )
    .await
    {
        Ok(message) => {
            if tool == "view_backlog" {
                fire_view_backlog_marker(&session);
            } else if !is_read_only_tool(&tool) {
                fire_board_invalidate(&session);
                fire_stories(&session, &refresh);
                if let (Some(id), Some(from_state)) = (transition_id, prior_state)
                    && let Some(target_state) =
                        declared_target_state.or_else(|| story_state_label(&refresh, id))
                    && target_state != from_state
                {
                    fire_story_transition(&session, id, from_state, target_state, &agent);
                }
            }
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
