use std::sync::Arc;

use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use store::routing::RoutingAction;

#[derive(Deserialize)]
pub struct RegisterRequest {
    #[serde(alias = "session_id")]
    pub user_id: String,
    pub display: String,
    #[serde(default)]
    pub terminal_id: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

pub async fn post_register(
    Extension(dispatch): Extension<store::Dispatch>,
    Extension(select): Extension<store::Select>,
    Extension(store): Extension<Arc<store::Store>>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    if roster::is_reserved_display(&payload.display) {
        return (
            StatusCode::FORBIDDEN,
            format!("'{}' is a reserved owner identity", payload.display),
        );
    }
    let superseded = payload
        .terminal_id
        .as_ref()
        .and_then(|terminal_id| {
            select.select(|state| store::routing::user_for_terminal(&state.routing, terminal_id))
        })
        .filter(|prior| *prior != payload.user_id);
    dispatch.dispatch(
        RoutingAction::Registered {
            user_id: payload.user_id.clone(),
            display: payload.display.clone(),
            role: payload.role.clone(),
        }
        .into(),
    );
    if let Some(terminal_id) = &payload.terminal_id {
        dispatch.dispatch(
            RoutingAction::TerminalLinked {
                user_id: payload.user_id.clone(),
                terminal_id: terminal_id.clone(),
            }
            .into(),
        );
        dispatch.dispatch(
            RoutingAction::Reaped {
                display: payload.display.clone(),
            }
            .into(),
        );
    }
    if let Some(prior) = superseded {
        crate::ws::dispatch_roster_leave(&store, &prior).await;
    }
    crate::ws::dispatch_roster_join(&store, &payload.user_id, &payload.display).await;
    (StatusCode::CREATED, "registered".to_string())
}
