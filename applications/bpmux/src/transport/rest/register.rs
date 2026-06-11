use std::sync::Arc;

use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::session::routing_identity::RoutingStore;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub session_id: String,
    pub display: String,
    #[serde(default)]
    pub terminal_id: Option<String>,
}

pub async fn post_register(
    Extension(routing): Extension<Arc<RoutingStore>>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    routing.register(&payload.session_id, &payload.display);
    if let Some(terminal_id) = &payload.terminal_id {
        routing.link_terminal(&payload.session_id, terminal_id);
    }
    (StatusCode::CREATED, "registered".to_string())
}
