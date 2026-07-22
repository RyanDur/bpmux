use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Extension, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;

use store::routing::RoutingAction;
use user_routing::short_name;

use super::shared::OWNER_NONCE_HEADER;

mod subscription_events;

use subscription_events::event_for_role;

pub const DEFAULT_WAIT_TIMEOUT_MS: u64 = 30_000;

#[derive(Deserialize)]
pub struct WaitParams {
    #[serde(rename = "as")]
    pub as_peer: Option<String>,
    pub after: Option<String>,
    pub timeout_ms: Option<u64>,
}

pub async fn wait_for_event(
    State(hub): State<Arc<hub::Hub>>,
    Extension(dispatch): Extension<store::Dispatch>,
    Extension(select): Extension<store::Select>,
    headers: HeaderMap,
    Query(params): Query<WaitParams>,
) -> impl IntoResponse {
    let role = params.as_peer.clone().unwrap_or_default();
    let resolved_ids =
        select.select(|state| store::routing::resolve(&state.routing, short_name(&role)));
    let claimed_owner_nonce =
        select.select(|state| owner_session::claimed_nonce(&state.owner_session));
    let presented_owner_nonce = headers
        .get(OWNER_NONCE_HEADER)
        .and_then(|value| value.to_str().ok());
    let is_owner = params.as_peer.is_some()
        && owner_session::resolve_owner_mark(
            claimed_owner_nonce.as_deref(),
            presented_owner_nonce,
            &role,
        );
    drain_or_block(
        &hub,
        &dispatch,
        &select,
        &params,
        &role,
        &resolved_ids,
        is_owner,
    )
    .await
}

async fn drain_or_block(
    hub: &hub::Hub,
    dispatch: &store::Dispatch,
    select: &store::Select,
    params: &WaitParams,
    role: &str,
    resolved_ids: &[String],
    is_owner: bool,
) -> (StatusCode, String) {
    let timeout = Duration::from_millis(params.timeout_ms.unwrap_or(DEFAULT_WAIT_TIMEOUT_MS));
    let after_cursor = durable_drain_cursor(dispatch, select, role, params.after.as_deref())
        .as_deref()
        .and_then(|raw| raw.parse::<chrono::DateTime<chrono::Utc>>().ok());

    let mut receiver = hub.subscribe();

    if let Some(event) = first_pending_after(hub, after_cursor, role, resolved_ids, is_owner) {
        return (StatusCode::OK, event.to_string());
    }

    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            received = receiver.recv() => match received {
                Ok(message) => {
                    if let Some(event) = event_for_role(&message, role, resolved_ids, is_owner) {
                        return (StatusCode::OK, event.to_string());
                    }
                }
                Err(RecvError::Lagged(_)) => {
                    if let Some(event) = first_pending_after(hub, after_cursor, role, resolved_ids, is_owner) {
                        return (StatusCode::OK, event.to_string());
                    }
                }
                Err(RecvError::Closed) => return timed_out(),
            },
            _ = &mut deadline => return timed_out(),
        }
    }
}

fn durable_drain_cursor(
    dispatch: &store::Dispatch,
    select: &store::Select,
    role: &str,
    client_after: Option<&str>,
) -> Option<String> {
    match select
        .select(|state| store::routing::resolve(&state.routing, short_name(role)))
        .into_iter()
        .next()
    {
        Some(user_id) => {
            let floor = select.select(|state| {
                store::routing::drain_floor(&state.routing, &user_id, client_after)
            });
            if let Some(ref position) = floor {
                dispatch.dispatch(
                    RoutingAction::CursorAdvanced {
                        user_id: user_id.clone(),
                        cursor: position.clone(),
                    }
                    .into(),
                );
            }
            floor
        }
        None => client_after.map(str::to_string),
    }
}

fn timed_out() -> (StatusCode, String) {
    (
        StatusCode::OK,
        serde_json::json!({ "timed_out": true }).to_string(),
    )
}

fn first_pending_after(
    hub: &hub::Hub,
    after_cursor: Option<chrono::DateTime<chrono::Utc>>,
    role: &str,
    resolved_ids: &[String],
    is_owner: bool,
) -> Option<serde_json::Value> {
    const PENDING_SCAN_WINDOW: usize = 500;
    let mut messages = hub.read_recent(PENDING_SCAN_WINDOW, 0, &|_| true).ok()?;
    messages.reverse();
    messages
        .into_iter()
        .filter(|message| match after_cursor {
            Some(cursor) => message.ts > cursor,
            None => true,
        })
        .find_map(|message| event_for_role(&message, role, resolved_ids, is_owner))
}
