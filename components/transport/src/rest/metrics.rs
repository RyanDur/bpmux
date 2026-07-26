use std::sync::Arc;

use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::board_metrics;
use super::shared::json_headers;
use product_management::data_gateway::TransitionsDataGateway;

pub fn assemble_metrics(
    select: &store::Select,
    transitions: &TransitionsDataGateway,
) -> serde_json::Value {
    let now = chrono::Utc::now();
    let window_start = now - chrono::Duration::days(view::velocity::WINDOW_DAYS);
    let events: Vec<view::velocity::Transition> = transitions
        .since(window_start.timestamp())
        .into_iter()
        .filter_map(|(story_id, new_state, at)| {
            chrono::DateTime::from_timestamp(at, 0).map(|at| view::velocity::Transition {
                story_id,
                new_state,
                at,
            })
        })
        .collect();
    let flow = view::velocity::compute(&events, now, |_| true);

    let (shared, local, iterations, feature_ids) = select.select(|state| {
        (
            state.shared_history.clone(),
            state.local_history.clone(),
            product_management::iteration::all(&state.iterations),
            product_management::story::slice::feature_ids(&state.stories),
        )
    });
    let board = board_metrics::build_velocity_view(&shared, &local, &iterations, |id| {
        feature_ids.contains(&id)
    });

    let mut payload = serde_json::to_value(&flow).unwrap_or_else(|_| serde_json::json!({}));
    payload["board"] = serde_json::to_value(&board).unwrap_or_else(|_| serde_json::json!({}));
    payload
}

pub async fn get_metrics(
    Extension(select): Extension<store::Select>,
    Extension(transitions): Extension<Arc<TransitionsDataGateway>>,
) -> impl IntoResponse {
    let payload = assemble_metrics(&select, &transitions);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}
