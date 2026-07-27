use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::board::{board_iterations, ordered_board_stories, sort_ready_by_membership};
use super::shared::{TrackerHandle, err_body, json_headers};

pub async fn get_board(
    axum::Extension(tracker_cell): axum::Extension<super::shared::TrackerCell>,
    axum::Extension(orchestrator): axum::Extension<sync::OrchestratorHandle>,
    axum::Extension(select): axum::Extension<store::Select>,
) -> impl IntoResponse {
    let tracker: TrackerHandle = tracker_cell.current();
    if let Some(refresh) = orchestrator.as_ref() {
        if select.select(|state| product_management::story::slice::is_empty(&state.stories)) {
            match refresh.refresh_all().await {
                Ok(()) => {
                    if select
                        .select(|state| product_management::story::slice::all(&state.stories))
                        .is_empty()
                    {
                        diag::log_diag(
                            "rest get_board: empty store; refresh_all returned Ok with 0 stories — ingest succeeded but source-of-truth empty OR decode dropped all rows",
                        );
                    } else {
                        diag::log_diag(&format!(
                            "rest get_board: empty store; refresh_all recovered {} stories",
                            select
                                .select(|state| product_management::story::slice::all(
                                    &state.stories
                                ))
                                .len()
                        ));
                    }
                }
                Err(e) => {
                    diag::log_diag(&format!(
                        "rest get_board: empty store; refresh_all failed: {e}"
                    ));
                }
            }
        }
        let body = serde_json::json!({
            "stories": ordered_board_stories(&select),
            "columns": super::board::board_columns_json(),
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
            let ordered = sort_ready_by_membership(stories, &[]);
            let body = serde_json::json!({
                "stories": ordered,
                "columns": super::board::board_columns_json(),
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

pub async fn get_iterations(
    axum::Extension(orchestrator): axum::Extension<sync::OrchestratorHandle>,
    axum::Extension(select): axum::Extension<store::Select>,
) -> impl IntoResponse {
    let Some(refresh) = orchestrator.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("iterations view requires the canonical store"),
        );
    };
    if select.select(|state| product_management::story::slice::is_empty(&state.stories)) {
        let _ = refresh.refresh_all().await;
    }
    let body = serde_json::json!({
        "iterations": board_iterations(&select),
        "fetched_at": chrono::Utc::now().to_rfc3339(),
    });
    (StatusCode::OK, json_headers(), body.to_string())
}

pub async fn get_story(
    axum::Extension(tracker_cell): axum::Extension<super::shared::TrackerCell>,
    axum::extract::Path(number): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let tracker: TrackerHandle = tracker_cell.current();
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
