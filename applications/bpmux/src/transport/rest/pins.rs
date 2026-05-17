//! Pins REST (#342) — `GET/POST/DELETE /api/pins`.
//!
//! State is per-session at `<session-dir>/pins.json` via
//! `session::pins`. Writes dispatch `PinsAction::PinAdded` /
//! `PinRemoved` to the orchestrator, which fans out a typed
//! `StateDelta::Pins` over WS so peer `mirror.pins` updates within
//! WS latency.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::{err_body, json_headers};
use crate::session;

#[derive(Deserialize)]
pub struct PinsChannelQuery {
    pub channel: String,
}

#[derive(Deserialize)]
pub struct PinCreate {
    pub channel: String,
    pub message_key: String,
    pub from: String,
    pub ts: String,
    pub text: String,
    pub pinned_by: String,
}

#[derive(Deserialize)]
pub struct PinDelete {
    pub channel: String,
    pub message_key: String,
}

pub async fn get_pins(
    State(session): State<Arc<session::Session>>,
    Query(params): Query<PinsChannelQuery>,
) -> impl IntoResponse {
    let store = session::pins::load(&session.dir);
    let pins = session::pins::list_pins(&store, &params.channel);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&pins).unwrap_or_else(|_| "[]".to_string()),
    )
}

pub async fn post_pin(
    State(session): State<Arc<session::Session>>,
    axum::Extension(orchestrator): axum::Extension<
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    >,
    axum::Json(body): axum::Json<PinCreate>,
) -> impl IntoResponse {
    let mut store = session::pins::load(&session.dir);
    let pin = session::pins::Pin {
        message_key: body.message_key.clone(),
        from: body.from,
        ts: body.ts.clone(),
        text: body.text,
        pinned_by: body.pinned_by.clone(),
        pinned_at: chrono::Utc::now().to_rfc3339(),
    };
    let added = session::pins::add_pin(&mut store, &body.channel, pin.clone());
    if added {
        if let Err(e) = session::pins::save(&session.dir, &store) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("save pins: {e}")),
            );
        }
        orchestrator
            .dispatch(crate::orchestrator::OrchestratorAction::Pins(
                crate::orchestrator::PinsAction::PinAdded {
                    channel: body.channel.clone(),
                    pin: crate::orchestrator::PinnedMessage {
                        message_key: body.message_key,
                        pinner: body.pinned_by,
                        ts: body.ts,
                        text: pin.text.clone(),
                        from: pin.from.clone(),
                        pinned_at: pin.pinned_at.clone(),
                    },
                },
            ))
            .await;
    }
    (
        StatusCode::OK,
        json_headers(),
        serde_json::json!({ "added": added }).to_string(),
    )
}

pub async fn delete_pin(
    State(session): State<Arc<session::Session>>,
    axum::Extension(orchestrator): axum::Extension<
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    >,
    axum::Json(body): axum::Json<PinDelete>,
) -> impl IntoResponse {
    let mut store = session::pins::load(&session.dir);
    let removed = session::pins::remove_pin(&mut store, &body.channel, &body.message_key);
    if removed {
        if let Err(e) = session::pins::save(&session.dir, &store) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("save pins: {e}")),
            );
        }
        orchestrator
            .dispatch(crate::orchestrator::OrchestratorAction::Pins(
                crate::orchestrator::PinsAction::PinRemoved {
                    channel: body.channel.clone(),
                    message_key: body.message_key.clone(),
                },
            ))
            .await;
    }
    (
        StatusCode::OK,
        json_headers(),
        serde_json::json!({ "removed": removed }).to_string(),
    )
}
