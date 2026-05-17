use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::session;

mod iteration_close;
mod lessons;
mod messages;
mod pins;
mod reactions;
mod search;
mod shared;

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
pub(crate) use shared::json_headers;
pub use shared::{CommentPayload, TrackerHandle};
use shared::{broadcast_board_invalidate, err_body};

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

// ── Board + story REST endpoints (owner-gate) ──────────────────────
//
// The Electron renderer fetches board state and performs owner-gate
// actions (accept / reject) through these endpoints. Every route in
// this section sits behind the bearer+loopback auth middleware that
// already guards `simulate_*`. On a single-token loopback deployment,
// an authorized caller is implicitly the local owner — there is no
// per-role gate beyond authentication. If the deployment model ever
// grows multi-user auth, a sender-identity check would go here.

/// GET /api/board — return the whole board as `{ stories, fetched_at }`.
pub async fn get_board(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
) -> impl IntoResponse {
    if let Some(refresh) = test_refresh.as_ref() {
        let mut all = refresh.all_stories();
        if all.is_empty() {
            match refresh.refresh_all().await {
                Ok(()) => {
                    all = refresh.all_stories();
                    if all.is_empty() {
                        crate::session::diag::log_diag(
                            "rest get_board: empty store; refresh_all returned Ok with 0 stories — ingest succeeded but source-of-truth empty OR decode dropped all rows",
                        );
                    } else {
                        crate::session::diag::log_diag(&format!(
                            "rest get_board: empty store; refresh_all recovered {} stories",
                            all.len()
                        ));
                    }
                }
                Err(e) => {
                    crate::session::diag::log_diag(&format!(
                        "rest get_board: empty store; refresh_all failed: {e}"
                    ));
                    all = refresh.all_stories();
                }
            }
        }
        let codec = crate::board::adapters::gh_label_codec::GhLabelCodec;
        let stories: Vec<serde_json::Value> = all
            .iter()
            .map(|s| crate::board::adapters::json::story_to_board_json(&codec, s))
            .collect();
        let ordered =
            crate::board::ready_order_label::sort_board_by_ready_order_labels(&codec, stories);
        let body = serde_json::json!({
            "stories": ordered,
            "fetched_at": chrono::Utc::now().to_rfc3339(),
        });
        return (StatusCode::OK, json_headers(), body.to_string());
    }
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.fetch_board_stories().await {
        Ok(stories) => {
            let codec = crate::board::adapters::gh_label_codec::GhLabelCodec;
            let ordered =
                crate::board::ready_order_label::sort_board_by_ready_order_labels(&codec, stories);
            let body = serde_json::json!({
                "stories": ordered,
                "fetched_at": chrono::Utc::now().to_rfc3339(),
            });
            (StatusCode::OK, json_headers(), body.to_string())
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("board fetch failed: {e}")),
        ),
    }
}

/// GET /api/stories/:number — return one story with full body.
pub async fn get_story(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.get_story(number).await {
        Ok(Some(story)) => (StatusCode::OK, json_headers(), story.to_string()),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            json_headers(),
            err_body(&format!("story #{number} not found")),
        ),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("story fetch failed: {e}")),
        ),
    }
}

#[derive(Deserialize)]
pub struct AcceptPayload {
    pub would_file_bug: bool,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub bypass_reason: Option<String>,
}

pub async fn post_accept_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<AcceptPayload>,
) -> impl IntoResponse {
    let mut args = serde_json::Map::new();
    args.insert(
        "would_file_bug".to_string(),
        serde_json::Value::Bool(payload.would_file_bug),
    );
    args.insert("force".to_string(), serde_json::Value::Bool(payload.force));
    if let Some(r) = payload.bypass_reason.as_ref() {
        args.insert(
            "bypass_reason".to_string(),
            serde_json::Value::String(r.clone()),
        );
    }
    let bypass = match crate::bridge::tools_stories::verify_bug_question_or_bypass(
        &args,
        crate::bridge::tools_stories::BugGate::Accept,
    ) {
        Ok(b) => b,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, json_headers(), err_body(&e));
        }
    };
    let Some(refresh) = test_refresh.as_ref().cloned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("orchestrator not configured"),
        );
    };
    match crate::bridge::tools_stories::accept_via_canonical_store(
        &refresh,
        number,
        "ui-electron".to_string(),
        bypass,
    )
    .await
    {
        Ok(()) => {
            broadcast_board_invalidate(&session);
            (
                StatusCode::OK,
                json_headers(),
                serde_json::json!({ "ok": true }).to_string(),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("accept failed: {e}")),
        ),
    }
}

#[derive(Deserialize)]
pub struct RejectPayload {
    pub note: String,
}

pub async fn post_reject_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<RejectPayload>,
) -> impl IntoResponse {
    let note = payload.note.trim().to_string();
    if note.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body("note must not be empty"),
        );
    }
    let Some(refresh) = test_refresh.as_ref().cloned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("orchestrator not configured"),
        );
    };
    match crate::bridge::tools_stories::reject_via_canonical_store(
        &refresh,
        number,
        "ui-electron".to_string(),
        note,
    )
    .await
    {
        Ok(()) => {
            broadcast_board_invalidate(&session);
            (
                StatusCode::OK,
                json_headers(),
                serde_json::json!({ "ok": true }).to_string(),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("reject failed: {e}")),
        ),
    }
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

/// GET /api/stories/:number/comments — list comments for a story.
pub async fn get_story_comments(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.list_story_comments(number).await {
        Ok(comments) => (
            StatusCode::OK,
            json_headers(),
            serde_json::to_string(&comments).unwrap_or_else(|_| "[]".to_string()),
        ),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("list comments failed: {e}")),
        ),
    }
}

/// GET /api/stories/:number/timeline — issue creation + event history
/// for the story's drill-in timeline section (#367). Returns condensed
/// event records the renderer classifies into lifecycle gates.
pub async fn get_story_timeline(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        if test_refresh.is_some() {
            return (StatusCode::OK, json_headers(), "[]".to_string());
        }
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.list_story_timeline(number).await {
        Ok(events) => (
            StatusCode::OK,
            json_headers(),
            serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string()),
        ),
        Err(e) => {
            if test_refresh.is_some() {
                return (StatusCode::OK, json_headers(), "[]".to_string());
            }
            (
                StatusCode::BAD_GATEWAY,
                json_headers(),
                err_body(&format!("list timeline failed: {e}")),
            )
        }
    }
}

/// POST /api/stories/:number/comments — add a comment.
pub async fn post_story_comment(
    State(session): State<Arc<session::Session>>,
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<CommentPayload>,
) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    if payload.body.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body("body must not be empty"),
        );
    }
    match tracker.post_story_comment(number, &payload.body).await {
        Ok(()) => {
            broadcast_board_invalidate(&session);
            (
                StatusCode::OK,
                json_headers(),
                serde_json::json!({ "ok": true }).to_string(),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("comment failed: {e}")),
        ),
    }
}

#[derive(Deserialize)]
pub struct LabelMutationPayload {
    #[serde(default)]
    pub add: Vec<String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

/// POST /api/stories/:number/labels — atomic add/remove label mutation
/// via the canonical mutation chain. Fires BoardEvent::TagRemoved for
/// each label in `remove` then TagAdded for each in `add`. Renderer-
/// driven so the caller controls the exact post-state of the story's
/// label-set (e.g., drag-within-iteration computes the new position-
/// suffix client-side via iterationPosition.ts util and POSTs the
/// resulting add/remove pair).
pub async fn post_story_labels(
    State(session): State<Arc<session::Session>>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<LabelMutationPayload>,
) -> impl IntoResponse {
    let Some(refresh) = test_refresh.as_ref().cloned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("orchestrator not configured"),
        );
    };
    for label in &payload.remove {
        let meta = crate::board::orchestrator::cache_fill_meta();
        if let Err(e) = refresh.mutate_sync(crate::board::BoardEvent::TagRemoved {
            meta,
            number,
            label: label.clone(),
        }) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("TagRemoved {label} on #{number}: {e}")),
            );
        }
    }
    for label in &payload.add {
        let meta = crate::board::orchestrator::cache_fill_meta();
        if let Err(e) = refresh.mutate_sync(crate::board::BoardEvent::TagAdded {
            meta,
            number,
            label: label.clone(),
        }) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("TagAdded {label} on #{number}: {e}")),
            );
        }
    }
    broadcast_board_invalidate(&session);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::json!({ "ok": true, "number": number }).to_string(),
    )
}
