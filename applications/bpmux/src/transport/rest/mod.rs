use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::session;

mod board;
mod iteration_close;
mod lessons;
mod messages;
mod pins;
mod reactions;
mod search;
mod shared;

pub use board::{
    AcceptPayload, LabelMutationPayload, RejectPayload, get_board, get_story, get_story_comments,
    get_story_timeline, post_accept_story, post_reject_story, post_story_comment,
    post_story_labels,
};
pub use iteration_close::{CloseIterationRequest, post_close_iteration};
#[allow(unused_imports)]
pub(crate) use iteration_close::{
    IterationCloseBlocker, collect_ready_queue_blockers, promote_members_then_close_iteration,
};
pub use lessons::{
    LessonsQuery, PeerLessonsQuery, SaveLessonPayload, get_lessons, get_peer_lessons, post_lesson,
};
pub use messages::{
    DEFAULT_MESSAGE_LIMIT, GetMessagesParams, PostMessage, get_messages, post_message,
};
pub use pins::{PinCreate, PinDelete, PinsChannelQuery, delete_pin, get_pins, post_pin};
#[allow(unused_imports)]
pub(crate) use reactions::{
    EXCERPT_MAX_LEN, build_reaction_event_text_with_excerpt, find_message_excerpt, truncate_excerpt,
};
pub use reactions::{ReactionToggle, ReactionsChannelQuery, get_reactions, post_reaction};
pub use search::{SearchParams, search_messages};
use shared::err_body;
pub(crate) use shared::json_headers;
pub use shared::{CommentPayload, TrackerHandle};

pub async fn get_screen(
    axum::Extension(capture): axum::Extension<Arc<std::sync::Mutex<String>>>,
) -> impl IntoResponse {
    let screen = capture
        .lock()
        .map(|s| s.clone())
        .unwrap_or_else(|_| "screen unavailable".to_string());
    (StatusCode::OK, screen)
}

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

/// Classifier misfire feedback endpoint (#159).
/// Writes a MISFIRE line to the diag log for later analysis. No effect
/// on the classifier itself — this is purely a training-signal capture
/// path so agent knowledge about wrong fires gets externalized instead
/// of silently dismissed.
pub async fn post_classifier_misfire(
    State(_session): State<Arc<session::Session>>,
    Json(payload): Json<ClassifierMisfire>,
) -> impl IntoResponse {
    let reason_field = payload
        .reason
        .as_deref()
        .map(|r| format!(" reason={r:?}"))
        .unwrap_or_default();
    // All user-controlled fields use the Debug formatter so newlines,
    // quotes, format-string artifacts, and NUL bytes get escaped into
    // the log line — prevents log forging / downstream parser attacks
    // via crafted `agent` or `nudge_hash` values.
    session::diag::log_diag(&format!(
        "MISFIRE agent={:?} nudge_hash={:?}{reason_field}",
        payload.agent, payload.nudge_hash
    ));
    (StatusCode::CREATED, "misfire recorded".to_string())
}

/// #402: serialized roster entry. Mirrors the frontend's
/// `RosterEntry` shape in `useTeamRoster.ts` — identity + derived
/// short-role. No system senders here (bpmux broadcasts system
/// messages but never holds a WS connection, so it can't enter
/// the roster via the peer-join path); frontend relies on this
/// invariant to skip the isSystem filter.
#[derive(serde::Serialize)]
pub struct RosterEntry {
    pub identity: String,
    pub role: String,
}

/// #402: snapshot of currently-connected WS peers. Source of truth
/// for the sidebar's directs + combinatorial-groups membership.
/// Empty array when no peers are connected (e.g., fresh start
/// before Electron's own WS handshake completes).
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

/// GET /api/ci — current CI state for the main branch + last-run
/// metadata. Reuses the MCP bridge's CI report builder so the
/// Electron CI glyph + MCP `get_ci_state` always agree by
/// construction.
pub async fn get_ci(
    axum::Extension(ci_state_store): axum::Extension<
        crate::transport::ci_state_store::CiStateStore,
    >,
) -> impl IntoResponse {
    let report = ci_state_store
        .read()
        .unwrap_or_else(crate::bridge::tools::unknown_ci_state_report);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string()),
    )
}

pub async fn get_ci_log(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::Extension(ci_log_store): axum::Extension<crate::transport::ci_log_store::CiLogStore>,
    axum::extract::Path(run_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    if let Some(cached) = ci_log_store.read(run_id) {
        return (
            StatusCode::OK,
            json_headers(),
            serde_json::to_string(&cached).unwrap_or_else(|_| "{}".to_string()),
        );
    }
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.fetch_failed_tests_for_run(run_id).await {
        Ok(failures) => {
            ci_log_store.write(run_id, failures.clone());
            (
                StatusCode::OK,
                json_headers(),
                serde_json::to_string(&failures).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(err) => {
            let status = match &err {
                crate::bridge::tools::CiLogError::RunNotFound { .. } => StatusCode::NOT_FOUND,
                crate::bridge::tools::CiLogError::RunInProgress { .. } => StatusCode::ACCEPTED,
                crate::bridge::tools::CiLogError::RateLimited { .. } => {
                    StatusCode::TOO_MANY_REQUESTS
                }
                crate::bridge::tools::CiLogError::UpstreamError { .. } => StatusCode::BAD_GATEWAY,
            };
            (
                status,
                json_headers(),
                serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string()),
            )
        }
    }
}

pub async fn post_ci_rerun(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::extract::Path(run_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.rerun_failed_jobs(run_id).await {
        Ok(()) => (
            StatusCode::OK,
            json_headers(),
            serde_json::to_string(&serde_json::json!({
                "status": "success",
                "run_id": run_id,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            serde_json::to_string(&serde_json::json!({
                "status": "failure",
                "run_id": run_id,
                "message": err,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
    }
}

/// Query params for GET /api/electron-capture: optional `role` for
/// the agent calling, used in the namespaced filename.
#[derive(Debug, Default, Deserialize)]
pub struct ElectronCaptureQuery {
    pub role: Option<String>,
}

/// GET /api/electron-capture — capture the bpmux Electron renderer
/// window via macOS `screencapture` and return the resulting PNG
/// path (#1183). Serialized via `electron_capture_lock` so concurrent
/// callers don't fight `screencapture` (PM-lean #3).
pub async fn get_electron_capture(
    axum::Extension(lock): axum::Extension<std::sync::Arc<tokio::sync::Mutex<()>>>,
    Query(params): Query<ElectronCaptureQuery>,
) -> impl IntoResponse {
    use crate::transport::electron_capture;
    let _guard = lock.lock().await;
    let dir = match electron_capture::captures_dir() {
        Ok(d) => d,
        Err(e) => return error_response(&e),
    };
    electron_capture::cleanup_old_captures(&dir);
    let window = match electron_capture::find_bpmux_window().await {
        Ok(w) => w,
        Err(e) => return error_response(&e),
    };
    let now = chrono::Utc::now();
    let role = params.role.as_deref().unwrap_or("agent");
    let filename = electron_capture::build_capture_filename(role, now);
    let path = dir.join(&filename);
    if let Err(e) = electron_capture::capture_window_to_path(window.window_id, &path).await {
        return error_response(&e);
    }
    let result = electron_capture::CaptureResult {
        path: path.to_string_lossy().into_owned(),
        window_id: window.window_id,
        captured_at: now.to_rfc3339(),
        width: window.width,
        height: window.height,
    };
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()),
    )
}

fn error_response(
    err: &crate::transport::electron_capture::CaptureError,
) -> (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    use crate::transport::electron_capture::CaptureError;
    let status = match err {
        CaptureError::ElectronNotRunning | CaptureError::NoWindow => StatusCode::NOT_FOUND,
        CaptureError::WindowMinimized => StatusCode::CONFLICT,
        CaptureError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
        CaptureError::SwiftUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        CaptureError::CaptureFailed { .. } | CaptureError::OutputWriteFailed { .. } => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    (
        status,
        json_headers(),
        serde_json::to_string(err).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// GET /api/metrics — velocity + DORA computed from the session log.
/// Returns JSON mirroring `session::velocity::VelocityMetrics` so the
/// Electron renderer can display without re-deriving.
pub async fn get_metrics(State(session): State<Arc<session::Session>>) -> impl IntoResponse {
    let messages = match session.read_messages() {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("read failed: {e}")),
            );
        }
    };
    let now = chrono::Utc::now();
    let metrics = crate::session::velocity::compute(&messages, now, |_| true);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&metrics).unwrap_or_else(|_| "{}".to_string()),
    )
}
