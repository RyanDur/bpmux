use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::shared::{err_body, json_headers};

pub async fn get_metrics(State(hub): State<Arc<hub::Hub>>) -> impl IntoResponse {
    let now = chrono::Utc::now();
    let window_start = now - chrono::Duration::days(view::velocity::WINDOW_DAYS);
    let messages = match hub.read_messages_since(window_start) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("read failed: {e}")),
            );
        }
    };

    let metrics = view::velocity::compute(&messages, now, |_| true);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&metrics).unwrap_or_else(|_| "{}".to_string()),
    )
}
