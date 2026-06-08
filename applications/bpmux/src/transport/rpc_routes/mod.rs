use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde_json::Value;

use crate::board::RefreshDriver;
use crate::bridge;
use crate::server::TestRefreshHandle;
use crate::session;

mod state_delta;
use state_delta::{
    fire_board_invalidate, fire_story_transition, fire_view_backlog_marker, story_state_label,
};

pub const AGENT_HEADER: &str = "X-BPMUX-Agent";

const KNOWN_AGENTS: &[&str] = &["dev", "pm", "designer", "owner"];

fn known_tools() -> &'static [&'static str] {
    &[
        "start_story",
        "deliver_story",
        "pass_desk_check",
        "fail_desk_check",
        "accept_story",
        "reject_story",
        "close_story",
        "create_story",
        "create_iteration",
        "close_iteration",
        "comment_on_story",
        "label_story",
        "set_parent_epic",
        "unset_parent_epic",
        "add_story_to_iteration",
        "designate_current_iteration",
        "reorder_story",
        "edit_story",
        "view_board",
        "get_story",
        "list_ready",
        "list_epic_children",
        "dev_in_flight",
        "view_story_timeline",
        "view_comments",
        "view_backlog",
        "submit_verdict",
        "view_verdicts",
    ]
}

fn is_read_only_tool(tool: &str) -> bool {
    matches!(
        tool,
        "view_board"
            | "get_story"
            | "list_ready"
            | "list_epic_children"
            | "dev_in_flight"
            | "view_story_timeline"
            | "view_comments"
            | "view_backlog"
            | "view_verdicts"
    )
}

pub fn agent_from_headers(headers: &HeaderMap) -> Result<String, String> {
    let raw = headers
        .get(AGENT_HEADER)
        .ok_or_else(|| format!("{AGENT_HEADER} header required"))?
        .to_str()
        .map_err(|_| format!("{AGENT_HEADER} header is not valid UTF-8"))?
        .trim()
        .to_ascii_lowercase();
    if raw.is_empty() {
        return Err(format!("{AGENT_HEADER} header is empty"));
    }
    if !KNOWN_AGENTS.contains(&raw.as_str()) {
        return Err(format!(
            "{AGENT_HEADER} '{raw}' is not a known role; expected one of {KNOWN_AGENTS:?}"
        ));
    }
    Ok(raw)
}

fn args_from_value(body: Value) -> Result<serde_json::Map<String, Value>, String> {
    match body {
        Value::Object(map) => Ok(map),
        other => Err(format!(
            "RPC body must be a JSON object; got {}",
            match other {
                Value::Null => "null",
                Value::Bool(_) => "bool",
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Array(_) => "array",
                Value::Object(_) => unreachable!(),
            }
        )),
    }
}

async fn dispatch(
    refresh: &Arc<dyn RefreshDriver>,
    tool: &str,
    agent: &str,
    args: serde_json::Map<String, Value>,
    session: &Arc<session::Session>,
) -> Result<String, String> {
    let refresh_ref: Option<&Arc<dyn RefreshDriver>> = Some(refresh);
    match tool {
        "start_story" => bridge::dispatch_test_start_story(args, agent, refresh_ref, session).await,
        "deliver_story" => {
            bridge::dispatch_test_deliver_story(args, agent, refresh_ref, session).await
        }
        "pass_desk_check" => {
            bridge::dispatch_test_pass_desk_check(args, agent, refresh_ref, session).await
        }
        "fail_desk_check" => {
            bridge::dispatch_test_fail_desk_check(args, agent, refresh_ref, session).await
        }
        "accept_story" => {
            bridge::dispatch_test_accept_story(args, agent, refresh_ref, session).await
        }
        "reject_story" => {
            bridge::dispatch_test_reject_story(args, agent, refresh_ref, session).await
        }
        "close_story" => bridge::dispatch_test_close_story(args, agent, refresh_ref).await,
        "create_story" => bridge::dispatch_test_create_story(args, agent, refresh_ref).await,
        "create_iteration" => {
            bridge::dispatch_test_create_iteration(args, agent, refresh_ref).await
        }
        "close_iteration" => {
            bridge::dispatch_test_close_iteration(args, agent, refresh_ref, session).await
        }
        "comment_on_story" => {
            bridge::dispatch_test_comment_on_story(args, agent, refresh_ref).await
        }
        "label_story" => bridge::dispatch_test_label_story(args, agent, refresh_ref).await,
        "set_parent_epic" => bridge::dispatch_test_set_parent_epic(args, agent, refresh_ref).await,
        "unset_parent_epic" => {
            bridge::dispatch_test_unset_parent_epic(args, agent, refresh_ref).await
        }
        "add_story_to_iteration" => {
            bridge::dispatch_test_add_story_to_iteration(args, agent, refresh_ref).await
        }
        "designate_current_iteration" => {
            bridge::dispatch_test_designate_current_iteration(args, agent, refresh_ref).await
        }
        "reorder_story" => bridge::dispatch_test_reorder_story(args, agent, refresh_ref).await,
        "move_story" => bridge::dispatch_test_move_story(args, agent, refresh_ref).await,
        "edit_story" => bridge::dispatch_test_edit_story(args, agent, refresh_ref).await,
        "view_board" => bridge::dispatch_test_view_board(args, refresh_ref).await,
        "get_story" => bridge::dispatch_test_get_story(args, refresh_ref).await,
        "list_ready" => bridge::dispatch_test_list_ready(refresh_ref).await,
        "list_epic_children" => bridge::dispatch_test_list_epic_children(args, refresh_ref).await,
        "dev_in_flight" => bridge::dispatch_test_dev_in_flight(refresh_ref).await,
        "view_story_timeline" => bridge::dispatch_test_view_story_timeline(args, refresh_ref).await,
        "view_comments" => bridge::dispatch_test_view_comments(args, refresh_ref).await,
        "view_backlog" => bridge::dispatch_test_view_backlog(refresh_ref).await,
        "submit_verdict" => bridge::dispatch_test_submit_verdict(args, agent, session).await,
        "view_verdicts" => bridge::dispatch_test_view_verdicts(args, session, refresh_ref).await,
        _ => Err(format!(
            "rpc: unknown tool '{tool}'; expected one of {:?}",
            known_tools()
        )),
    }
}

pub async fn post_rpc_tool(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
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
    let transition_number = args.get("number").and_then(|v| v.as_u64());
    let declared_target_state = if tool == "move_story" {
        args.get("target_state")
            .and_then(|v| v.as_str())
            .and_then(crate::board::domain::StoryState::parse)
            .map(|s| s.as_str())
    } else {
        None
    };
    let prior_state = transition_number
        .filter(|_| !is_read_only_tool(&tool))
        .and_then(|number| story_state_label(&refresh, number));
    match dispatch(&refresh, &tool, &agent, args, &session).await {
        Ok(message) => {
            if tool == "view_backlog" {
                fire_view_backlog_marker(&session);
            } else if !is_read_only_tool(&tool) {
                fire_board_invalidate(&session);
                if let (Some(number), Some(from_state)) = (transition_number, prior_state)
                    && let Some(target_state) =
                        declared_target_state.or_else(|| story_state_label(&refresh, number))
                    && target_state != from_state
                {
                    fire_story_transition(&session, number, from_state, target_state, &agent);
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn header_map(values: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (k, v) in values {
            headers.insert(*k, HeaderValue::from_str(v).expect("test header"));
        }
        headers
    }

    #[test]
    fn agent_from_headers_accepts_known_role_lowercase() {
        let h = header_map(&[(AGENT_HEADER, "dev")]);
        assert_eq!(agent_from_headers(&h).unwrap(), "dev");
    }

    #[test]
    fn agent_from_headers_canonicalizes_case_and_whitespace() {
        let h = header_map(&[(AGENT_HEADER, "  PM  ")]);
        assert_eq!(agent_from_headers(&h).unwrap(), "pm");
    }

    #[test]
    fn agent_from_headers_rejects_missing_header() {
        let err = agent_from_headers(&HeaderMap::new()).unwrap_err();
        assert!(
            err.contains("required"),
            "expected 'required' in {err}, got {err:?}"
        );
    }

    #[test]
    fn agent_from_headers_rejects_unknown_role() {
        let h = header_map(&[(AGENT_HEADER, "intern")]);
        let err = agent_from_headers(&h).unwrap_err();
        assert!(
            err.contains("not a known role"),
            "expected 'not a known role' in {err}, got {err:?}"
        );
    }

    #[test]
    fn agent_from_headers_rejects_empty_value() {
        let h = header_map(&[(AGENT_HEADER, "   ")]);
        let err = agent_from_headers(&h).unwrap_err();
        assert!(
            err.contains("empty"),
            "expected 'empty' in {err}, got {err:?}"
        );
    }

    #[test]
    fn args_from_value_accepts_object() {
        let body = serde_json::json!({"a": 1, "b": "two"});
        let args = args_from_value(body).unwrap();
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn args_from_value_rejects_non_object() {
        let body = serde_json::json!([1, 2]);
        let err = args_from_value(body).unwrap_err();
        assert!(err.contains("must be a JSON object"));
    }

    #[test]
    fn known_tools_covers_full_state_machine_and_critical_reads() {
        let tools = known_tools();
        for required in [
            "start_story",
            "deliver_story",
            "pass_desk_check",
            "fail_desk_check",
            "accept_story",
            "reject_story",
            "close_story",
            "view_board",
            "get_story",
            "list_ready",
        ] {
            assert!(
                tools.contains(&required),
                "{required} must be in known_tools (load-bearing for canonical-store cutover)"
            );
        }
    }

    #[test]
    fn is_read_only_tool_distinguishes_mutations_from_queries() {
        assert!(is_read_only_tool("view_board"));
        assert!(is_read_only_tool("get_story"));
        assert!(!is_read_only_tool("start_story"));
        assert!(!is_read_only_tool("deliver_story"));
    }
}
