use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::shared::{err_body, json_headers};
use crate::session;

#[derive(serde::Serialize)]
pub struct RosterEntry {
    pub identity: String,
    pub role: String,
}

pub async fn get_roster(State(session): State<Arc<session::Session>>) -> impl IntoResponse {
    let roster: Vec<RosterEntry> = session
        .roster_snapshot()
        .into_iter()
        .map(|identity| {
            let role = identity.split(':').next().unwrap_or(&identity).to_string();
            RosterEntry { identity, role }
        })
        .collect();
    let body = serde_json::json!({
        "roster": roster,
        "fetched_at": chrono::Utc::now().to_rfc3339(),
    });
    (StatusCode::OK, json_headers(), body.to_string())
}

pub async fn get_sync_status(
    axum::Extension(refresh): axum::Extension<crate::server::TestRefreshHandle>,
) -> impl IntoResponse {
    let status = match refresh {
        Some(r) => r.snapshot_status(),
        None => crate::board::SyncStatus::default(),
    };
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_string()),
    )
}

pub async fn post_force_sync(
    axum::Extension(refresh): axum::Extension<crate::server::TestRefreshHandle>,
) -> impl IntoResponse {
    let Some(r) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("orchestrator not configured"),
        );
    };
    match r.drain_pending().await {
        Ok(n) => (
            StatusCode::OK,
            json_headers(),
            format!("{{\"drained\":{n}}}"),
        ),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("force-sync failed: {e}")),
        ),
    }
}

pub async fn post_acknowledge_dropped(
    axum::Extension(refresh): axum::Extension<crate::server::TestRefreshHandle>,
) -> impl IntoResponse {
    let Some(r) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("orchestrator not configured"),
        );
    };
    let cleared = r.acknowledge_dropped();
    (
        StatusCode::OK,
        json_headers(),
        format!("{{\"acknowledged\":{cleared}}}"),
    )
}
