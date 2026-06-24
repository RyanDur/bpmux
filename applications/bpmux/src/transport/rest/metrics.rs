use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::shared::{err_body, json_headers};
use crate::session;

pub async fn get_metrics(State(session): State<Arc<session::Session>>) -> impl IntoResponse {
    let messages = match session.read_messages() {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("read failed: {e}")),
            );
        }
    };
    let now = chrono::Utc::now();
    let metrics = crate::board::velocity::compute(&messages, now, |_| true);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&metrics).unwrap_or_else(|_| "{}".to_string()),
    )
}
