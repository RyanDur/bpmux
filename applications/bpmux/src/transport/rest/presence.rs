use std::sync::Arc;

use axum::Json;
use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::orchestrator::stores::Presence;
use crate::routing::RoutingStore;

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
    Extension(routing): Extension<Arc<RoutingStore>>,
    Extension(orchestrator): Extension<
        Arc<
            crate::orchestrator::Orchestrator<
                crate::product_management::board::adapters::github::GitHubSourceAdapter,
            >,
        >,
    >,
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
    let routing_ids = routing.resolve(&agent);
    if routing_ids.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            format!("no connected session registered for agent '{agent}'"),
        );
    }
    for routing_id in &routing_ids {
        crate::transport::ws::dispatch_roster_presence(&orchestrator, routing_id, presence).await;
    }
    (
        StatusCode::OK,
        format!("{agent} presence set to {presence:?}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_from_state_accepts_the_three_agent_pushable_states() {
        assert_eq!(presence_from_state("busy"), Some(Presence::Busy));
        assert_eq!(presence_from_state("idle"), Some(Presence::Idle));
        assert_eq!(presence_from_state("help"), Some(Presence::Help));
    }

    #[test]
    fn presence_from_state_is_case_and_whitespace_insensitive() {
        assert_eq!(
            presence_from_state("  HELP "),
            Some(Presence::Help),
            "the verb normalizes like agent_from_headers does for the role"
        );
    }

    #[test]
    fn presence_from_state_rejects_states_an_agent_must_not_self_assign() {
        assert_eq!(
            presence_from_state("online"),
            None,
            "online is connection-derived (join sets it), not agent-pushable"
        );
        assert_eq!(
            presence_from_state("offline"),
            None,
            "offline is connection-derived (disconnect), not agent-pushable"
        );
        assert_eq!(
            presence_from_state("blocked"),
            None,
            "blocked is the owner-park's to set, not the agent's"
        );
        assert_eq!(presence_from_state("nonsense"), None);
    }
}
