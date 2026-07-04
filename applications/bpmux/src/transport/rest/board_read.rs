use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::board::{board_iterations, ordered_board_stories};
use super::shared::{TrackerHandle, err_body, json_headers};

pub async fn get_board(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::Extension(orchestrator): axum::Extension<crate::sync::OrchestratorHandle>,
) -> impl IntoResponse {
    if let Some(refresh) = orchestrator.as_ref() {
        if refresh.store_is_empty() {
            match refresh.refresh_all().await {
                Ok(()) => {
                    if refresh.all_stories().is_empty() {
                        crate::diag::log_diag(
                            "rest get_board: empty store; refresh_all returned Ok with 0 stories — ingest succeeded but source-of-truth empty OR decode dropped all rows",
                        );
                    } else {
                        crate::diag::log_diag(&format!(
                            "rest get_board: empty store; refresh_all recovered {} stories",
                            refresh.all_stories().len()
                        ));
                    }
                }
                Err(e) => {
                    crate::diag::log_diag(&format!(
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
            let ordered =
                crate::view::board::ready_order_label::sort_board_ready_by_membership(stories, &[]);
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

pub async fn get_iterations(
    axum::Extension(orchestrator): axum::Extension<crate::sync::OrchestratorHandle>,
) -> impl IntoResponse {
    let Some(refresh) = orchestrator.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("iterations view requires the canonical store"),
        );
    };
    if refresh.store_is_empty() {
        let _ = refresh.refresh_all().await;
    }
    let body = serde_json::json!({
        "iterations": board_iterations(refresh),
        "fetched_at": chrono::Utc::now().to_rfc3339(),
    });
    (StatusCode::OK, json_headers(), body.to_string())
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
