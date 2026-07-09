use std::sync::Arc;

use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::store::routing::RoutingAction;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub session_id: String,
    pub display: String,
    #[serde(default)]
    pub terminal_id: Option<String>,
}

pub async fn post_register(
    Extension(dispatch): Extension<crate::store::Dispatch>,
    Extension(select): Extension<crate::store::Select>,
    Extension(store): Extension<Arc<crate::store::Store>>,
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
        .and_then(|terminal_id| {
            select.select(|s| crate::store::routing::session_for_terminal(&s.routing, terminal_id))
        })
        .filter(|prior| *prior != payload.session_id);
    dispatch.dispatch(
        RoutingAction::Registered {
            session_id: payload.session_id.clone(),
            display: payload.display.clone(),
        }
        .into(),
    );
    if let Some(terminal_id) = &payload.terminal_id {
        dispatch.dispatch(
            RoutingAction::TerminalLinked {
                session_id: payload.session_id.clone(),
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
        crate::transport::ws::dispatch_roster_leave(&store, &prior).await;
    }
    crate::transport::ws::dispatch_roster_join(&store, &payload.session_id, &payload.display).await;
    (StatusCode::CREATED, "registered".to_string())
}
