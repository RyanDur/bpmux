//! Classifier misfire feedback endpoint (#159) — write-only training-
//! signal capture path so agent knowledge about wrong classifier fires
//! gets externalized to the diag log instead of silently dismissed.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::session;

/// POST body for the classifier misfire feedback endpoint (#159).
#[derive(Deserialize)]
pub struct ClassifierMisfire {
    pub agent: String,
    pub nudge_hash: String,
    /// Optional per the acceptance criteria — agents can mark a fire
    /// as wrong without explaining why. Per Naur/Schmidt: state the
    /// observation, not the conclusion.
    #[serde(default)]
    pub reason: Option<String>,
}

/// POST /api/classifier-misfire — writes a `MISFIRE` line to the diag
/// log for later analysis. No effect on the classifier itself — this
/// is purely a training-signal capture path so agent knowledge about
/// wrong fires gets externalized instead of silently dismissed.
///
/// All user-controlled fields use the `Debug` formatter so newlines,
/// quotes, format-string artifacts, and NUL bytes get escaped into
/// the log line — prevents log forging / downstream parser attacks
/// via crafted `agent` or `nudge_hash` values.
pub async fn post_classifier_misfire(
    State(_session): State<Arc<session::Session>>,
    Json(payload): Json<ClassifierMisfire>,
) -> impl IntoResponse {
    let reason_field = payload
        .reason
        .as_deref()
        .map(|r| format!(" reason={r:?}"))
        .unwrap_or_default();
    session::diag::log_diag(&format!(
        "MISFIRE agent={:?} nudge_hash={:?}{reason_field}",
        payload.agent, payload.nudge_hash
    ));
    (StatusCode::CREATED, "misfire recorded".to_string())
}
