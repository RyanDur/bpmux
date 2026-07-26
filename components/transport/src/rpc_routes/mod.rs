use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde_json::Value;

use sync::OrchestratorHandle;

mod state_delta;
mod tool_dispatch;
mod validation;

#[cfg(test)]
mod effective_role_tests;
#[cfg(test)]
mod rpc_dispatch_tests;

use state_delta::{
    fire_board_invalidate, fire_stories, fire_story_transition, fire_view_backlog_marker,
    story_state_label,
};
use tool_dispatch::{ToolCaller, dispatch, effective_role};
pub use validation::{ACTOR_ID_HEADER, AGENT_HEADER, actor_id_from_headers, agent_from_headers};
use validation::{args_from_value, is_read_only_tool};

#[allow(clippy::too_many_arguments)]
pub async fn post_rpc_tool(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(refresh): axum::Extension<OrchestratorHandle>,
    axum::Extension(dispatcher): axum::Extension<store::Dispatch>,
    axum::Extension(select): axum::Extension<store::Select>,
    axum::Extension(race_window): axum::Extension<sync::RaceWindowHandle>,
    axum::Extension(transitions): axum::Extension<
        std::sync::Arc<product_management::data_gateway::TransitionsDataGateway>,
    >,
    axum::Extension(ci_runs): axum::Extension<std::sync::Arc<ci::data_gateway::CiRunsDataGateway>>,
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
    let agent = workflow::current_workflow()
        .role_name_for_purpose(&agent)
        .map(str::to_string)
        .unwrap_or(agent);
    let actor_id = actor_id_from_headers(&headers);
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
                use product_management::scheduling::ProjectedState;
                match raw.to_ascii_lowercase().as_str() {
                    "icebox" => Some(ProjectedState::Icebox.as_str()),
                    "ready" => Some(ProjectedState::Ready.as_str()),
                    other => product_management::story::Work::parse(other)
                        .filter(|s| *s != product_management::story::Work::Ready)
                        .map(|s| s.as_str()),
                }
            })
    } else {
        None
    };
    let prior_state = transition_id
        .filter(|_| !is_read_only_tool(&tool))
        .and_then(|id| story_state_label(&select, id));
    let role = effective_role(&select, &agent, actor_id.as_deref());
    let caller = ToolCaller {
        agent: &agent,
        role: &role,
        actor_id: actor_id.as_deref(),
    };
    match dispatch(
        &refresh,
        &tool,
        caller,
        args,
        &hub,
        &dispatcher,
        &select,
        &race_window,
        &transitions,
        &ci_runs,
    )
    .await
    {
        Ok(message) => {
            if tool == "view_backlog" {
                fire_view_backlog_marker(&hub);
            } else if !is_read_only_tool(&tool) {
                fire_board_invalidate(&hub);
                fire_stories(&hub, &select);
                if let (Some(id), Some(from_state)) = (transition_id, prior_state)
                    && let Some(target_state) =
                        declared_target_state.or_else(|| story_state_label(&select, id))
                    && target_state != from_state
                {
                    fire_story_transition(&hub, id, from_state, target_state, &agent);
                }
            }
            (
                StatusCode::OK,
                Json(serde_json::json!({ "message": message })),
            )
        }
        Err(e) => {
            diag::log_diag(&format!("rpc: {tool} rejected: {e}"));
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
        }
    }
}
