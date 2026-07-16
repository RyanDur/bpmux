use std::sync::Arc;

use axum::Json;
use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::store::{Presence, Select};

#[derive(Deserialize)]
pub struct PresenceRequest {
    pub state: String,
}

fn presence_from_state(state: &str) -> Option<Presence> {
    match state.trim().to_ascii_lowercase().as_str() {
        "busy" => Some(Presence::Busy),
        "idle" => Some(Presence::Idle),
        "help" => Some(Presence::Help),
        _ => None,
    }
}

pub async fn post_presence(
    headers: HeaderMap,
    Extension(select): Extension<Select>,
    Extension(store): Extension<Arc<crate::store::Store>>,
    Json(payload): Json<PresenceRequest>,
) -> impl IntoResponse {
    let agent = match crate::transport::rpc_routes::agent_from_headers(&headers) {
        Ok(a) => a,
        Err(e) => return (StatusCode::BAD_REQUEST, e),
    };
    let Some(presence) = presence_from_state(&payload.state) else {
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "unknown presence state '{}'; an agent pushes one of busy|idle|help",
                payload.state
            ),
        );
    };
    let routing_ids = select.select(|state| crate::store::routing::resolve(&state.routing, &agent));
    if routing_ids.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            format!("no connected user registered for agent '{agent}'"),
        );
    }
    for routing_id in &routing_ids {
        crate::transport::ws::dispatch_roster_presence(&store, routing_id, presence).await;
    }
    (
        StatusCode::OK,
        format!("{agent} presence set to {presence:?}"),
    )
}
