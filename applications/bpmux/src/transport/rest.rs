use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::session;

pub const DEFAULT_MESSAGE_LIMIT: usize = 20;

#[derive(Deserialize)]
pub struct PostMessage {
    pub from: String,
    pub text: String,
    pub message_type: Option<String>,
    /// Attention signal (#158) — role this message is directed at.
    /// See `session::message::Message::addressee`.
    #[serde(default)]
    pub addressee: Option<String>,
    /// Multi-depth rendering payload (#187). Optional; omitted
    /// requests produce `multi_depth: None` on the broadcast
    /// message, which is identical to pre-#187 behavior.
    #[serde(default)]
    pub multi_depth: Option<session::message::MultiDepthPayload>,
    /// Privacy filter — optional list of recipient names.
    /// When set, only the listed roles receive the message.
    #[serde(default)]
    pub to: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct GetMessagesParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    /// Filter to chat messages only (excludes activity/system). Default: true.
    pub chat_only: Option<bool>,
    /// Simulate delivery-side filtering as a specific peer (#191).
    /// When set, only returns messages that would be delivered to this
    /// role via WebSocket.
    #[serde(rename = "as")]
    pub as_peer: Option<String>,
}

pub async fn get_messages(
    State(session): State<Arc<session::Session>>,
    Query(params): Query<GetMessagesParams>,
) -> impl IntoResponse {
    match session.read_messages() {
        Ok(messages) => {
            let chat_only = params.chat_only.unwrap_or(true);
            let offset = params.offset.unwrap_or(0);
            let limit = params.limit.unwrap_or(DEFAULT_MESSAGE_LIMIT);
            let mut filtered: Vec<_> = if chat_only {
                messages.into_iter().filter(|m| m.is_chat()).collect()
            } else {
                messages
            };
            if let Some(ref peer) = params.as_peer {
                filtered.retain(|m| super::ws::should_deliver(m, peer));
            }
            filtered.reverse();
            let page: Vec<_> = filtered.into_iter().skip(offset).take(limit).collect();
            (
                StatusCode::OK,
                serde_json::to_string(&page).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read: {e}"),
        ),
    }
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub sender: Option<String>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub channel: Option<String>,
    pub message_type: Option<String>,
}

pub async fn search_messages(
    State(session): State<Arc<session::Session>>,
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    // Consolidate all three searches (messages, story bodies, story
    // comments) into one RTT per #344 AC7. Frontend gets the whole
    // result set in one fetch; no progressive reveal. Stories +
    // comments both need the tracker — when tracker is None (no
    // GH_TOKEN) they return empty arrays and the renderer just shows
    // 0 results in those sections without a 503.
    let messages_part = match session.read_messages() {
        Ok(all_messages) => {
            let message_class = params.message_type.as_deref().and_then(|t| match t {
                "correction" | "experience" => Some(session::MessageClass::Experience(
                    session::classifier::ExperienceKind::Correction,
                )),
                "decision" => Some(session::MessageClass::Decision),
                "discussion" => Some(session::MessageClass::Discussion),
                _ => None,
            });
            let filters = session::SearchFilters {
                query: params.query.clone(),
                sender: params.sender.clone(),
                after: params.after.as_ref().and_then(|s| s.parse().ok()),
                before: params.before.as_ref().and_then(|s| s.parse().ok()),
                channel: params.channel.clone(),
                message_class,
                insight_only: params.message_type.as_deref() == Some("insight"),
                owner_name: session.owner_name.clone(),
            };
            session::search(&all_messages, &filters)
        }
        Err(_) => Vec::new(),
    };

    let (stories_part, comments_part) = search_stories_and_comments(&tracker, &params.query).await;

    let body = serde_json::json!({
        "messages": messages_part,
        "stories": stories_part,
        "comments": comments_part,
    });
    (
        StatusCode::OK,
        serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// Substring-match the query against story titles/bodies + comment
/// bodies. Case-insensitive, minimal ranking. Returns
/// (story_hits, comment_hits). Empty vecs when tracker is None —
/// caller doesn't surface a different error for missing config.
async fn search_stories_and_comments(
    tracker: &TrackerHandle,
    query: &str,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let Some(tracker) = tracker else {
        return (Vec::new(), Vec::new());
    };
    if query.trim().is_empty() {
        return (Vec::new(), Vec::new());
    }
    let needle = query.to_lowercase();

    let stories = match tracker.fetch_board_stories().await {
        Ok(s) => s,
        Err(_) => return (Vec::new(), Vec::new()),
    };

    // Story hits: substring match on title OR body. snippet extracts
    // ~80 chars around the first body match so the renderer has a
    // preview line without needing to fetch the full body.
    let mut story_hits: Vec<serde_json::Value> = Vec::new();
    for s in &stories {
        let title = s.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let body = s.get("body").and_then(|v| v.as_str()).unwrap_or("");
        let title_match = title.to_lowercase().contains(&needle);
        let body_match = body.to_lowercase().contains(&needle);
        if !title_match && !body_match {
            continue;
        }
        let snippet = if body_match {
            extract_snippet(body, &needle)
        } else {
            title.to_string()
        };
        story_hits.push(serde_json::json!({
            "number": s.get("number"),
            "title": title,
            "snippet": snippet,
            "matched_in": if title_match { "title" } else { "body" },
        }));
    }

    // Comment hits: N tracker.list_story_comments calls (one per
    // story). Run concurrently via join_all to collapse N×RTT into
    // one RTT-group. For bpmux scale (~20-50 stories) this is a
    // manageable fan-out; above that we'd want a server-side search
    // API on the tracker instead of client-side filtering.
    let story_numbers: Vec<u64> = stories
        .iter()
        .filter_map(|s| s.get("number").and_then(|n| n.as_u64()))
        .collect();
    let futures = story_numbers
        .iter()
        .map(|n| async move { (*n, tracker.list_story_comments(*n).await) });
    let results = futures::future::join_all(futures).await;
    let mut comment_hits: Vec<serde_json::Value> = Vec::new();
    for (story_number, res) in results {
        let Ok(comments) = res else { continue };
        for c in comments {
            let body = c.get("body").and_then(|v| v.as_str()).unwrap_or("");
            if !body.to_lowercase().contains(&needle) {
                continue;
            }
            comment_hits.push(serde_json::json!({
                "story_number": story_number,
                "comment_id": c.get("id"),
                "author": c.get("author"),
                "snippet": extract_snippet(body, &needle),
            }));
        }
    }

    (story_hits, comment_hits)
}

/// Return ~80 chars around the first needle occurrence, with ellipsis
/// where truncated. Falls back to the first 80 chars if the needle
/// isn't found (shouldn't happen post-contains but defensive).
fn extract_snippet(haystack: &str, needle: &str) -> String {
    let haystack_lower = haystack.to_lowercase();
    let Some(idx) = haystack_lower.find(needle) else {
        return haystack.chars().take(80).collect();
    };
    let window = 40;
    let start = idx.saturating_sub(window);
    let end = (idx + needle.len() + window).min(haystack.len());
    let mut snippet = String::new();
    if start > 0 {
        snippet.push('…');
    }
    snippet.push_str(&haystack[start..end]);
    if end < haystack.len() {
        snippet.push('…');
    }
    snippet
}

#[derive(Deserialize)]
pub struct LessonsQuery {
    pub topic: Option<String>,
    pub limit: Option<usize>,
}

pub async fn get_lessons(
    State(session): State<Arc<session::Session>>,
    Query(params): Query<LessonsQuery>,
) -> impl IntoResponse {
    match session.read_messages() {
        Ok(messages) => {
            let mut index = session::lessons::build_index(&messages, &session.owner_name);
            let saved = session.storage.load_lessons();
            index.lessons = saved.lessons;

            // If limit is set, return top N lessons ranked by reinforcement
            if let Some(limit) = params.limit {
                index.lessons.sort_by(|a, b| {
                    b.reinforcement_count
                        .cmp(&a.reinforcement_count)
                        .then_with(|| b.established.cmp(&a.established))
                });
                index.lessons.truncate(limit);
                let mut text = format!("{} top lessons\n", index.lessons.len());
                let mut watch_topics = Vec::new();
                for l in &index.lessons {
                    text.push_str(&format!(
                        "\n{} ({}): {}\n",
                        l.topic, l.author_role, l.lesson
                    ));
                    if !watch_topics.contains(&l.topic) {
                        watch_topics.push(l.topic.clone());
                    }
                }
                text.push_str(&format!("\nWatch topics: {}\n", watch_topics.join(", ")));
                return (StatusCode::OK, text);
            }

            let groups = session::lessons::group_by_topic(&index);
            let topic_filter = params.topic.as_deref().map(str::to_lowercase);
            let filtered: Vec<_> = groups
                .iter()
                .filter(|g| {
                    topic_filter
                        .as_ref()
                        .is_none_or(|f| g.topic.to_lowercase().contains(f))
                })
                .collect();
            let mut text = format!("{} topics indexed\n", filtered.len());
            for group in &filtered {
                text.push_str(&format!(
                    "\n{} ({} experiences, {} lessons)\n",
                    group.topic,
                    group.experiences.len(),
                    group.lessons.len(),
                ));
                for c in &group.experiences {
                    text.push_str(&format!("  {}: {}\n", &c.timestamp[..10], c.text_preview));
                }
                for l in &group.lessons {
                    text.push_str(&format!("  {} learned: {}\n", l.author_role, l.lesson));
                }
            }
            (StatusCode::OK, text)
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read: {e}"),
        ),
    }
}

#[derive(Deserialize)]
pub struct SaveLessonPayload {
    pub topic: String,
    pub lesson: String,
    pub author_role: String,
    pub evidence: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct PeerLessonsQuery {
    pub peer: Option<String>,
}

pub async fn get_peer_lessons(
    State(session): State<Arc<session::Session>>,
    Query(params): Query<PeerLessonsQuery>,
) -> impl IntoResponse {
    let Some(peer_name) = params.peer else {
        let peers = session.storage.list_peers();
        return (
            StatusCode::OK,
            format!("available peers: {}", peers.join(", ")),
        );
    };

    let index = session.storage.load_peer_lessons(&peer_name);
    let groups = session::lessons::group_by_topic(&index);
    if groups.is_empty() {
        return (
            StatusCode::OK,
            format!("No lessons from peer \"{peer_name}\"."),
        );
    }

    let mut text = format!("{} topics from {peer_name} (read-only)\n", groups.len());
    for group in &groups {
        text.push_str(&format!(
            "\n{} ({} experiences, {} lessons)\n",
            group.topic,
            group.experiences.len(),
            group.lessons.len(),
        ));
        for l in &group.lessons {
            text.push_str(&format!("  {} learned: {}\n", l.author_role, l.lesson));
        }
    }
    (StatusCode::OK, text)
}

pub async fn get_screen(
    axum::Extension(capture): axum::Extension<Arc<std::sync::Mutex<String>>>,
) -> impl IntoResponse {
    let screen = capture
        .lock()
        .map(|s| s.clone())
        .unwrap_or_else(|_| "screen unavailable".to_string());
    (StatusCode::OK, screen)
}

#[derive(Deserialize)]
pub struct ClickPayload {
    pub x: u16,
    pub y: u16,
}

pub async fn simulate_click(
    axum::Extension(tx): axum::Extension<
        tokio::sync::mpsc::Sender<crate::terminal::event::TerminalInjectEvent>,
    >,
    Json(payload): Json<ClickPayload>,
) -> impl IntoResponse {
    let event = crate::terminal::event::TerminalInjectEvent::Click {
        x: payload.x,
        y: payload.y,
    };
    match tx.send(event).await {
        Ok(()) => (
            StatusCode::OK,
            format!("click at ({}, {})", payload.x, payload.y),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "event loop not available".to_string(),
        ),
    }
}

#[derive(Deserialize)]
pub struct KeyPayload {
    pub key: String,
    /// Modifier names to apply to the key event. Accepted values
    /// (case-insensitive): `ctrl` / `control`, `alt`, `shift`,
    /// `super`. Unknown modifier strings produce a `400 Bad Request`
    /// rather than being silently dropped — agents need loud
    /// feedback instead of a silently-ignored keystroke.
    #[serde(default)]
    pub modifiers: Vec<String>,
}

pub async fn simulate_key(
    axum::Extension(tx): axum::Extension<
        tokio::sync::mpsc::Sender<crate::terminal::event::TerminalInjectEvent>,
    >,
    Json(payload): Json<KeyPayload>,
) -> impl IntoResponse {
    let key_display = payload.key.clone();
    let event = crate::terminal::event::TerminalInjectEvent::Key {
        key: payload.key,
        modifiers: payload.modifiers,
    };
    // Validate the vocabulary at the transport boundary so a bad
    // key/modifier name surfaces as a 400 before anything enters
    // the channel.
    if let Err(e) = event.validate() {
        return (StatusCode::BAD_REQUEST, e.message());
    }
    match tx.send(event).await {
        Ok(()) => (StatusCode::OK, format!("key: {key_display}")),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "event loop not available".to_string(),
        ),
    }
}

#[derive(Deserialize)]
pub struct TypePayload {
    pub text: String,
}

pub async fn simulate_type(
    axum::Extension(tx): axum::Extension<
        tokio::sync::mpsc::Sender<crate::terminal::event::TerminalInjectEvent>,
    >,
    Json(payload): Json<TypePayload>,
) -> impl IntoResponse {
    let len = payload.text.chars().count();
    let event = crate::terminal::event::TerminalInjectEvent::Type { text: payload.text };
    match tx.send(event).await {
        Ok(()) => (StatusCode::OK, format!("typed {len} chars")),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "event loop not available".to_string(),
        ),
    }
}

pub async fn simulate_input_clear(
    axum::Extension(tx): axum::Extension<
        tokio::sync::mpsc::Sender<crate::terminal::event::TerminalInjectEvent>,
    >,
) -> impl IntoResponse {
    match tx
        .send(crate::terminal::event::TerminalInjectEvent::Clear)
        .await
    {
        Ok(()) => (StatusCode::OK, "input cleared".to_string()),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "event loop not available".to_string(),
        ),
    }
}

pub async fn simulate_input_submit(
    axum::Extension(tx): axum::Extension<
        tokio::sync::mpsc::Sender<crate::terminal::event::TerminalInjectEvent>,
    >,
) -> impl IntoResponse {
    match tx
        .send(crate::terminal::event::TerminalInjectEvent::Submit)
        .await
    {
        Ok(()) => (StatusCode::OK, "submit dispatched".to_string()),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "event loop not available".to_string(),
        ),
    }
}

pub async fn post_lesson(
    State(session): State<Arc<session::Session>>,
    Json(payload): Json<SaveLessonPayload>,
) -> impl IntoResponse {
    match session.storage.add_lesson(
        &payload.topic,
        &payload.lesson,
        &payload.author_role,
        payload.evidence.unwrap_or_default(),
    ) {
        Ok(()) => (StatusCode::CREATED, "lesson saved".to_string()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to save: {e}"),
        ),
    }
}

pub async fn post_message(
    State(session): State<Arc<session::Session>>,
    Json(payload): Json<PostMessage>,
) -> impl IntoResponse {
    if payload.text.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "text must not be empty".to_string(),
        );
    }
    if payload.text.len() > 65536 {
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "message too large: {} bytes exceeds limit of 65536",
                payload.text.len()
            ),
        );
    }
    // Reject externally-originated `system` messages. System messages
    // are reserved for internal bridge signals (classifier nudges,
    // handoff warnings, join/leave) and must not be spoofable by HTTP
    // callers. Internal callers that emit system messages do so
    // through session::log::append / emit_system, not this handler.
    if payload.message_type.as_deref() == Some("system") {
        return (
            StatusCode::FORBIDDEN,
            "message_type=\"system\" is reserved for internal callers".to_string(),
        );
    }
    // A `to` filter on incoming POST means the caller chose a
    // directed recipient — same privacy semantic as `/to` in the
    // TUI. Sets explicit_privacy so downstream guards (e.g., /share
    // refusing to widen private messages) apply uniformly whether
    // the DM originated from an agent, a hook, or the owner.
    let explicit_privacy = payload.to.is_some();
    let msg = session::Message {
        from: payload.from,
        text: payload.text,
        ts: chrono::Utc::now(),
        message_type: session::MessageType::from(payload.message_type.as_deref()),
        to: payload.to,
        addressee: payload.addressee,
        explicit_privacy,
        multi_depth: payload.multi_depth,
    };
    if let Err(e) = session.broadcast(&msg) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to broadcast: {e}"),
        );
    }
    (StatusCode::CREATED, "message sent".to_string())
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

/// Optional shared tracker handle, injected via axum Extension at
/// server startup. `None` when GitHub env config is missing — the
/// handlers then return 503 so the renderer can surface "tracker not
/// configured" instead of a silent empty board.
pub type TrackerHandle = Option<Arc<crate::bridge::tracker::OctocrabStoryTracker>>;

/// Structured error body. All handlers in this section reply with
/// this shape on failure so the renderer can branch on the `error`
/// key without string-matching response text.
fn err_body(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

pub(super) fn json_headers() -> [(axum::http::HeaderName, &'static str); 1] {
    [(axum::http::header::CONTENT_TYPE, "application/json")]
}

/// Broadcast a board-invalidate marker through the session. Mirrors
/// the MCP bridge's `broadcast_board_invalidate` — same marker text,
/// same system message_type — so the TUI's event loop picks it up
/// and triggers an immediate board refetch regardless of which path
/// (MCP tool or HTTP REST) caused the mutation.
fn broadcast_board_invalidate(session: &session::Session) {
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: crate::bridge::BOARD_INVALIDATE_MARKER.to_string(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::System,
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!("rest: board-invalidate broadcast failed: {e}"));
    }
}

/// GET /api/board — return the whole board as `{ stories, fetched_at }`.
pub async fn get_board(
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.fetch_board_stories().await {
        Ok(stories) => {
            let body = serde_json::json!({
                "stories": stories,
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

/// POST /api/stories/:number/accept — accept a delivered story.
pub async fn post_accept_story(
    State(session): State<Arc<session::Session>>,
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
    match tracker.accept_story(number).await {
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

/// POST /api/stories/:number/reject — reject a delivered story with
/// a required non-empty note.
///
/// Note: extractor ordering matters for axum — body-consuming
/// extractors (Json) must come LAST. Path is keyed-arg, Extension +
/// State are shared, Json is body.
pub async fn post_reject_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<RejectPayload>,
) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    if payload.note.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body("note must not be empty"),
        );
    }
    match tracker.reject_story(number, &payload.note).await {
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

#[derive(Deserialize)]
pub struct CommentPayload {
    pub body: String,
}

/// GET /api/ci — current CI state for the main branch + last-run metadata.
/// Reuses the MCP bridge's CI report builder so the Electron CI glyph +
/// MCP get_ci_state always agree by construction.
/// Pins REST (#342) — GET/POST/DELETE /api/pins.
///
/// State is per-session at `<session-dir>/pins.json` via
/// `session::pins`. Writes broadcast `PINS_UPDATE_MARKER` so peer
/// renderers refetch without polling.
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
    axum::Json(body): axum::Json<PinCreate>,
) -> impl IntoResponse {
    let mut store = session::pins::load(&session.dir);
    let pin = session::pins::Pin {
        message_key: body.message_key,
        from: body.from,
        ts: body.ts,
        text: body.text,
        pinned_by: body.pinned_by,
        pinned_at: chrono::Utc::now().to_rfc3339(),
    };
    let added = session::pins::add_pin(&mut store, &body.channel, pin);
    if added {
        if let Err(e) = session::pins::save(&session.dir, &store) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("save pins: {e}")),
            );
        }
        broadcast_pins_update(&session);
    }
    (
        StatusCode::OK,
        json_headers(),
        serde_json::json!({ "added": added }).to_string(),
    )
}

pub async fn delete_pin(
    State(session): State<Arc<session::Session>>,
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
        broadcast_pins_update(&session);
    }
    (
        StatusCode::OK,
        json_headers(),
        serde_json::json!({ "removed": removed }).to_string(),
    )
}

fn broadcast_pins_update(session: &session::Session) {
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: crate::bridge::PINS_UPDATE_MARKER.to_string(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::System,
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!("rest: pins-update broadcast failed: {e}"));
    }
}

/// Reactions REST (#340) — GET/POST /api/reactions.
///
/// POST toggles: if the (user, emoji, message_key) triple exists,
/// it's removed; otherwise added. Single endpoint for add + remove
/// keeps the renderer's click-handler path trivial.
#[derive(Deserialize)]
pub struct ReactionsChannelQuery {
    pub channel: String,
}

#[derive(Deserialize)]
pub struct ReactionToggle {
    pub channel: String,
    pub message_key: String,
    pub emoji: String,
    pub user: String,
}

pub async fn get_reactions(
    State(session): State<Arc<session::Session>>,
    Query(params): Query<ReactionsChannelQuery>,
) -> impl IntoResponse {
    let store = session::reactions::load(&session.dir);
    let agg = session::reactions::aggregate_for_channel(&store, &params.channel);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&agg).unwrap_or_else(|_| "[]".to_string()),
    )
}

pub async fn post_reaction(
    State(session): State<Arc<session::Session>>,
    axum::Json(body): axum::Json<ReactionToggle>,
) -> impl IntoResponse {
    let mut store = session::reactions::load(&session.dir);
    let now = chrono::Utc::now().to_rfc3339();
    let present = session::reactions::toggle_reaction(
        &mut store,
        &body.channel,
        &body.message_key,
        &body.emoji,
        &body.user,
        &now,
    );
    if let Err(e) = session::reactions::save(&session.dir, &store) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(&format!("save reactions: {e}")),
        );
    }
    broadcast_reactions_update(&session);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::json!({ "present": present }).to_string(),
    )
}

fn broadcast_reactions_update(session: &session::Session) {
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: crate::bridge::REACTIONS_UPDATE_MARKER.to_string(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::System,
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!("rest: reactions-update broadcast failed: {e}"));
    }
}

pub async fn get_ci(axum::Extension(tracker): axum::Extension<TrackerHandle>) -> impl IntoResponse {
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    use crate::bridge::tracker::StoryTracker;
    match tracker.list_latest_run_on_main().await {
        Ok(latest) => {
            let runs: Vec<crate::terminal::ci::RunEntry> = latest
                .iter()
                .map(|w| crate::terminal::ci::RunEntry {
                    status: w.status.clone(),
                    conclusion: w.conclusion.clone(),
                    head_sha: w.head_sha.clone(),
                    display_title: w.display_title.clone(),
                    database_id: w.database_id,
                })
                .collect();
            let report = crate::bridge::tools::build_ci_state_report(&runs);
            (
                StatusCode::OK,
                json_headers(),
                serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("ci state failed: {e}")),
        ),
    }
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
pub struct ReorderPayload {
    pub state: String,
    pub position: usize,
}

/// POST /api/stories/:number/reorder — persist a story's queue
/// position within a state column. Writes through the existing
/// `session::priority` index (same durability path as the MCP
/// `reorder_story` tool); broadcasts board-invalidate so other
/// surfaces refresh.
pub async fn post_reorder_story(
    State(session): State<Arc<session::Session>>,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<ReorderPayload>,
) -> impl IntoResponse {
    let path = match crate::session::priority::default_priority_path() {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("priority path: {e}")),
            );
        }
    };
    let mut index = crate::session::priority::load_priority(&path);
    crate::session::priority::reorder(&mut index, &payload.state, number, payload.position);
    if let Err(e) = crate::session::priority::save_priority(&path, &index) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(&format!("save priority: {e}")),
        );
    }
    broadcast_board_invalidate(&session);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::json!({ "ok": true }).to_string(),
    )
}
