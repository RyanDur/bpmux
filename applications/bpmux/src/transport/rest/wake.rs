use std::sync::Arc;

use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::room::CommitmentStore;

#[derive(Deserialize)]
pub struct WakeOnEventRequest {
    pub role: String,
    pub event_class: String,
    #[serde(default)]
    pub filter: Option<Map<String, Value>>,
}

pub async fn wake_on_event(
    Extension(commitments): Extension<Arc<CommitmentStore>>,
    Json(payload): Json<WakeOnEventRequest>,
) -> impl IntoResponse {
    commitments.commit(&payload.role, &payload.event_class, payload.filter);
    (StatusCode::CREATED, "committed".to_string())
}
