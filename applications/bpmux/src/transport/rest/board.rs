use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::{
    CommentPayload, TrackerHandle, broadcast_board_invalidate, broadcast_board_snapshot, err_body,
    json_headers,
};
use crate::session;

pub(crate) fn ordered_board_stories(
    refresh: &Arc<dyn crate::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    let all = refresh.all_stories();
    let codec = crate::board::adapters::gh_label_codec::GhLabelCodec;
    let workflow = crate::workflow::current_workflow();
    let stories: Vec<serde_json::Value> = all
        .iter()
        .map(|s| {
            let mut json = crate::board::adapters::json::story_to_board_json(&codec, s);
            if s.story_type == crate::board::domain::StoryType::Iteration
                && let serde_json::Value::Object(map) = &mut json
            {
                let members: Vec<&crate::board::domain::Story> = all
                    .iter()
                    .filter(|m| m.iteration_membership.map(|(it, _)| it) == Some(s.number))
                    .collect();
                let readiness =
                    crate::board::iteration_readiness::iteration_readiness(s, &members, workflow);
                if let Ok(value) = serde_json::to_value(readiness) {
                    map.insert("readiness".to_string(), value);
                }
            }
            json
        })
        .collect();
    crate::board::ready_order_label::sort_board_ready_by_membership(stories)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::ordered_board_stories;
    use crate::board::RefreshDriver;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use crate::board::{BoardStore, SyncOrchestrator};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    async fn orchestrator(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories);
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
        orch.refresh_all().await.unwrap();
        orch
    }

    fn iteration(number: u64) -> Story {
        let mut s = Story::new_minimal(
            number,
            "Iteration Vela".to_string(),
            StoryState::Ready,
            Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
            "pm".into(),
        );
        s.story_type = StoryType::Iteration;
        s.body = Some("## Iteration overview\n\ncomposed".to_string());
        s.iteration_review_closed_by = vec!["designer".to_string()];
        s.iteration_dev_ok_by = vec!["dev".to_string()];
        s.raw_labels = vec![
            "iteration".into(),
            "ready".into(),
            "iteration-review-closed.designer".into(),
            "iteration-dev-ok.dev".into(),
        ];
        s
    }

    fn member(number: u64, iteration_number: u64, position: u32) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("member #{number}"),
            StoryState::Ready,
            Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
            "pm".into(),
        );
        s.iteration_membership = Some((iteration_number, position));
        s.raw_labels = vec![format!("parent-iteration-#{iteration_number}.{position}")];
        s
    }

    fn find(stories: &[serde_json::Value], number: u64) -> &serde_json::Value {
        stories
            .iter()
            .find(|s| s["number"].as_u64() == Some(number))
            .expect("story present in snapshot")
    }

    #[tokio::test]
    async fn iteration_snapshot_carries_computed_readiness_1703() {
        let orch = orchestrator(vec![iteration(2000), member(2001, 2000, 100)]).await;
        let stories = ordered_board_stories(&orch);
        let readiness = &find(&stories, 2000)["readiness"];
        assert_eq!(
            readiness["review_closed"].as_bool(),
            Some(true),
            "the designer review-close derives review_closed=true, exposed for #1704 to consume"
        );
        assert_eq!(readiness["drafted"].as_bool(), Some(true));
        assert_eq!(readiness["stories_ordered"].as_bool(), Some(true));
        assert_eq!(
            readiness["dev_ok"].as_bool(),
            Some(true),
            "the dev-OK derives dev_ok=true, exposed for #1704 to consume"
        );
    }

    #[tokio::test]
    async fn non_iteration_story_has_no_readiness_key_1703() {
        let orch = orchestrator(vec![member(2001, 2000, 100)]).await;
        let stories = ordered_board_stories(&orch);
        assert!(
            find(&stories, 2001).get("readiness").is_none(),
            "readiness is an iteration-only concern; feature stories carry no readiness key"
        );
    }
}

pub async fn get_board(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
) -> impl IntoResponse {
    if let Some(refresh) = test_refresh.as_ref() {
        if refresh.all_stories().is_empty() {
            match refresh.refresh_all().await {
                Ok(()) => {
                    if refresh.all_stories().is_empty() {
                        crate::session::diag::log_diag(
                            "rest get_board: empty store; refresh_all returned Ok with 0 stories — ingest succeeded but source-of-truth empty OR decode dropped all rows",
                        );
                    } else {
                        crate::session::diag::log_diag(&format!(
                            "rest get_board: empty store; refresh_all recovered {} stories",
                            refresh.all_stories().len()
                        ));
                    }
                }
                Err(e) => {
                    crate::session::diag::log_diag(&format!(
                        "rest get_board: empty store; refresh_all failed: {e}"
                    ));
                }
            }
        }
        let body = serde_json::json!({
            "stories": ordered_board_stories(refresh),
            "fetched_at": chrono::Utc::now().to_rfc3339(),
        });
        return (StatusCode::OK, json_headers(), body.to_string());
    }
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.fetch_board_stories().await {
        Ok(stories) => {
            let ordered = crate::board::ready_order_label::sort_board_ready_by_membership(stories);
            let body = serde_json::json!({
                "stories": ordered,
                "fetched_at": chrono::Utc::now().to_rfc3339(),
            });
            (StatusCode::OK, json_headers(), body.to_string())
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("board fetch failed: {e}")),
        ),
    }
}

pub async fn get_story(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.get_story(number).await {
        Ok(Some(story)) => (StatusCode::OK, json_headers(), story.to_string()),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            json_headers(),
            err_body(&format!("story #{number} not found")),
        ),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("story fetch failed: {e}")),
        ),
    }
}

#[derive(Deserialize)]
pub struct AcceptPayload {
    pub would_file_bug: bool,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub bypass_reason: Option<String>,
}

pub async fn post_accept_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<AcceptPayload>,
) -> impl IntoResponse {
    let mut args = serde_json::Map::new();
    args.insert(
        "would_file_bug".to_string(),
        serde_json::Value::Bool(payload.would_file_bug),
    );
    args.insert("force".to_string(), serde_json::Value::Bool(payload.force));
    if let Some(r) = payload.bypass_reason.as_ref() {
        args.insert(
            "bypass_reason".to_string(),
            serde_json::Value::String(r.clone()),
        );
    }
    let bypass = match crate::bridge::tools_stories::verify_bug_question_or_bypass(
        &args,
        crate::bridge::tools_stories::BugGate::Accept,
    ) {
        Ok(b) => b,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, json_headers(), err_body(&e));
        }
    };
    let Some(refresh) = test_refresh.as_ref().cloned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("orchestrator not configured"),
        );
    };
    match crate::bridge::tools_stories::accept_via_canonical_store(
        &refresh,
        number,
        "ui-electron".to_string(),
        bypass,
    )
    .await
    {
        Ok(()) => {
            broadcast_board_invalidate(&session);
            broadcast_board_snapshot(&session, &refresh);
            (
                StatusCode::OK,
                json_headers(),
                serde_json::json!({ "ok": true }).to_string(),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("accept failed: {e}")),
        ),
    }
}

#[derive(Deserialize)]
pub struct RejectPayload {
    pub note: String,
}

pub async fn post_reject_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<RejectPayload>,
) -> impl IntoResponse {
    let note = payload.note.trim().to_string();
    if note.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body("note must not be empty"),
        );
    }
    let Some(refresh) = test_refresh.as_ref().cloned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("orchestrator not configured"),
        );
    };
    let workflow = crate::workflow::current_workflow();
    let target_state = workflow
        .transition_for("reject", crate::workflow::GateClass::TeamVerifiable)
        .or_else(|| workflow.transition_by_id("reject"))
        .and_then(|t| crate::board::domain::StoryState::parse(&t.to))
        .unwrap_or(crate::board::domain::StoryState::Ready);
    match crate::bridge::tools_stories::reject_via_canonical_store(
        &refresh,
        number,
        "ui-electron".to_string(),
        note,
        target_state,
    )
    .await
    {
        Ok(()) => {
            broadcast_board_invalidate(&session);
            broadcast_board_snapshot(&session, &refresh);
            (
                StatusCode::OK,
                json_headers(),
                serde_json::json!({ "ok": true }).to_string(),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("reject failed: {e}")),
        ),
    }
}

pub async fn get_story_comments(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.list_story_comments(number).await {
        Ok(comments) => (
            StatusCode::OK,
            json_headers(),
            serde_json::to_string(&comments).unwrap_or_else(|_| "[]".to_string()),
        ),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("list comments failed: {e}")),
        ),
    }
}

pub async fn get_story_timeline(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        if test_refresh.is_some() {
            return (StatusCode::OK, json_headers(), "[]".to_string());
        }
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.list_story_timeline(number).await {
        Ok(events) => (
            StatusCode::OK,
            json_headers(),
            serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string()),
        ),
        Err(e) => {
            if test_refresh.is_some() {
                return (StatusCode::OK, json_headers(), "[]".to_string());
            }
            (
                StatusCode::BAD_GATEWAY,
                json_headers(),
                err_body(&format!("list timeline failed: {e}")),
            )
        }
    }
}

pub async fn post_story_comment(
    State(session): State<Arc<session::Session>>,
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<CommentPayload>,
) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    if payload.body.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body("body must not be empty"),
        );
    }
    match tracker.post_story_comment(number, &payload.body).await {
        Ok(()) => {
            broadcast_board_invalidate(&session);
            (
                StatusCode::OK,
                json_headers(),
                serde_json::json!({ "ok": true }).to_string(),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("comment failed: {e}")),
        ),
    }
}
