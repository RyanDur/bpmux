use std::sync::Arc;

use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::routing::RoutingStore;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub session_id: String,
    pub display: String,
    #[serde(default)]
    pub terminal_id: Option<String>,
}

pub async fn post_register(
    Extension(routing): Extension<Arc<RoutingStore>>,
    Extension(orchestrator): Extension<
        Arc<crate::orchestrator::Orchestrator<crate::integrations::github::GitHubSourceAdapter>>,
    >,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    if crate::transport::ws::is_reserved_display(&payload.display) {
        return (
            StatusCode::FORBIDDEN,
            format!("'{}' is a reserved owner identity", payload.display),
        );
    }
    let superseded = payload
        .terminal_id
        .as_ref()
        .and_then(|terminal_id| routing.session_for_terminal(terminal_id))
        .filter(|prior| *prior != payload.session_id);
    routing.register(&payload.session_id, &payload.display);
    if let Some(terminal_id) = &payload.terminal_id {
        routing.link_terminal(&payload.session_id, terminal_id);
        routing.reap_unclaimed_duplicates_of(&payload.display);
    }
    if let Some(prior) = superseded {
        crate::transport::ws::dispatch_roster_leave(&orchestrator, &prior).await;
    }
    crate::transport::ws::dispatch_roster_join(
        &orchestrator,
        &payload.session_id,
        &payload.display,
    )
    .await;
    (StatusCode::CREATED, "registered".to_string())
}
