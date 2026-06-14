use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, State};
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
    State(session): State<Arc<crate::session::Session>>,
    Extension(routing): Extension<Arc<RoutingStore>>,
    Extension(orchestrator): Extension<
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    >,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    if crate::transport::ws::is_reserved_display(&payload.display) {
        return (
            StatusCode::FORBIDDEN,
            format!("'{}' is a reserved owner identity", payload.display),
        );
    }
    let previous_display = payload
        .terminal_id
        .as_ref()
        .and_then(|terminal_id| routing.display_for_terminal(terminal_id));
    routing.register(&payload.session_id, &payload.display);
    if let Some(terminal_id) = &payload.terminal_id {
        routing.link_terminal(&payload.session_id, terminal_id);
        routing.reap_unclaimed_duplicates_of(&payload.display);
        let bridge_name = crate::session::routing_identity::bridge_agent_name(terminal_id);
        session.roster_join(&payload.display);
        crate::transport::ws::dispatch_roster_join(&orchestrator, &payload.display).await;
        for stale in previous_display.into_iter().chain([bridge_name]) {
            if stale != payload.display {
                session.roster_leave(&stale);
                crate::transport::ws::dispatch_roster_leave(&orchestrator, &stale).await;
            }
        }
    }
    (StatusCode::CREATED, "registered".to_string())
}
