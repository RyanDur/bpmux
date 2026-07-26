use std::sync::Arc;

use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::shared::json_headers;
use product_management::data_gateway::TransitionsDataGateway;

pub async fn get_metrics(
    Extension(transitions): Extension<Arc<TransitionsDataGateway>>,
) -> impl IntoResponse {
    let now = chrono::Utc::now();
    let window_start = now - chrono::Duration::days(view::velocity::WINDOW_DAYS);
    let rows = transitions.since(window_start.timestamp());
    let events: Vec<view::velocity::Transition> = rows
        .into_iter()
        .filter_map(|(story_id, new_state, at)| {
            chrono::DateTime::from_timestamp(at, 0).map(|at| view::velocity::Transition {
                story_id,
                new_state,
                at,
            })
        })
        .collect();

    let metrics = view::velocity::compute(&events, now, |_| true);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&metrics).unwrap_or_else(|_| "{}".to_string()),
    )
}
