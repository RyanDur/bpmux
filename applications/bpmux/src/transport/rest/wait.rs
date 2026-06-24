use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;

use crate::session;
use crate::session::routing_identity::RoutingStore;
use crate::transport::ws::should_deliver;

pub const DEFAULT_WAIT_TIMEOUT_MS: u64 = 30_000;

#[derive(Deserialize)]
pub struct WaitParams {
    #[serde(rename = "as")]
    pub as_peer: Option<String>,
    pub after: Option<String>,
    pub timeout_ms: Option<u64>,
}

pub async fn wait_for_event(
    State(session): State<Arc<session::Session>>,
    Extension(routing): Extension<Arc<RoutingStore>>,
    Query(params): Query<WaitParams>,
) -> impl IntoResponse {
    let role = params.as_peer.clone().unwrap_or_default();
    drain_or_block(&session, &routing, &params, &role).await
}

async fn drain_or_block(
    session: &session::Session,
    routing: &RoutingStore,
    params: &WaitParams,
    role: &str,
) -> (StatusCode, String) {
    let timeout = Duration::from_millis(params.timeout_ms.unwrap_or(DEFAULT_WAIT_TIMEOUT_MS));
    let after_cursor = durable_drain_cursor(routing, role, params.after.as_deref())
        .as_deref()
        .and_then(|raw| raw.parse::<chrono::DateTime<chrono::Utc>>().ok());

    let mut receiver = session.subscribe();

    if let Some(event) = first_pending_after(session, after_cursor, role) {
        return (StatusCode::OK, event.to_string());
    }

    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            received = receiver.recv() => match received {
                Ok(message) => {
                    if let Some(event) = event_for_role(&message, role) {
                        return (StatusCode::OK, event.to_string());
                    }
                }
                Err(RecvError::Lagged(_)) => {
                    if let Some(event) = first_pending_after(session, after_cursor, role) {
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
    routing: &RoutingStore,
    role: &str,
    client_after: Option<&str>,
) -> Option<String> {
    match routing.resolve(short_name(role)).into_iter().next() {
        Some(session_id) => routing.drain_floor(&session_id, client_after),
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
    session: &session::Session,
    after_cursor: Option<chrono::DateTime<chrono::Utc>>,
    role: &str,
) -> Option<serde_json::Value> {
    const PENDING_SCAN_WINDOW: usize = 500;
    let mut messages = session
        .read_recent(PENDING_SCAN_WINDOW, 0, &|_| true)
        .ok()?;
    messages.reverse();
    messages
        .into_iter()
        .filter(|message| match after_cursor {
            Some(cursor) => message.ts > cursor,
            None => true,
        })
        .find_map(|message| event_for_role(&message, role))
}

fn event_for_role(message: &crate::message::Message, role: &str) -> Option<serde_json::Value> {
    if short_name(&message.from) == short_name(role) {
        return None;
    }
    if let Some(delta) = &message.state_delta {
        let ts = message
            .ts
            .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
        let (event_class, payload) = delta.to_subscription_event(&ts)?;
        return Some(serde_json::json!({
            "event_class": event_class,
            "payload": payload,
            "cursor": ts,
        }));
    }
    if !message.is_chat() || !should_deliver(message, role) {
        return None;
    }
    Some(broadcast_event(message))
}

fn broadcast_event(message: &crate::message::Message) -> serde_json::Value {
    let url_safe_ts = message
        .ts
        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
    serde_json::json!({
        "event_class": "broadcast_received",
        "payload": {
            "from": short_name(&message.from),
            "text": message.text,
            "ts": url_safe_ts,
            "channel": channel_label(&message.to),
        },
        "cursor": url_safe_ts,
    })
}

fn channel_label(to: &Option<Vec<String>>) -> String {
    match to {
        None => "broadcast".to_string(),
        Some(recipients)
            if recipients
                .iter()
                .any(|recipient| recipient == crate::message::BROADCAST_ALL) =>
        {
            "broadcast".to_string()
        }
        Some(recipients) => recipients.join(","),
    }
}

fn short_name(name: &str) -> &str {
    name.split(':').next().unwrap_or(name)
}
