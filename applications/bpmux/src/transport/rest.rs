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
    /// Filter to exactly this message_type (#364). When set, overrides
    /// `chat_only`'s default exclusion so callers can hydrate presence
    /// (`message_type=activity`) without bumping `chat_only=false` and
    /// pulling unrelated message classes.
    pub message_type: Option<String>,
    /// #1153: paged-history support. When set, returns only messages
    /// with `ts < before`. Pairs with `limit` to page older history
    /// on client-side scroll-up. Format: RFC3339 timestamp string.
    pub before: Option<String>,
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
            let mut filtered: Vec<_> = if let Some(ref mt) = params.message_type {
                let wanted = session::MessageType::from(Some(mt.as_str()));
                messages
                    .into_iter()
                    .filter(|m| m.message_type == wanted)
                    .collect()
            } else if chat_only {
                messages.into_iter().filter(|m| m.is_chat()).collect()
            } else {
                messages
            };
            if let Some(ref peer) = params.as_peer {
                filtered.retain(|m| super::ws::should_deliver(m, peer));
            }
            // #1153: paged-history. When `before=<rfc3339>` is set,
            // strict-less-than filter on message timestamp so callers
            // can fetch older windows by passing the oldest-currently-
            // rendered timestamp. Strict-less-than avoids duplicates at
            // the boundary; client-side dedup handles any race.
            if let Some(ref before_str) = params.before
                && let Ok(before_ts) = before_str.parse::<chrono::DateTime<chrono::Utc>>()
            {
                filtered.retain(|m| m.ts < before_ts);
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
        state_delta: None,
        state_snapshot: None,
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
pub type TrackerHandle = Option<Arc<crate::board::adapters::github::GitHubSourceAdapter>>;

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
        let stories: Vec<serde_json::Value> = all
            .iter()
            .map(crate::board::domain::story_to_board_json)
            .collect();
        let (ordered, icebox_unified) = match crate::session::priority::default_priority_path() {
            Ok(path) => {
                let index = crate::session::priority::load_priority(&path);
                let icebox_unified = serde_json::to_value(&index.icebox_unified)
                    .unwrap_or_else(|_| serde_json::json!([]));
                (
                    crate::session::priority::apply_priority_to_board(stories, &index),
                    icebox_unified,
                )
            }
            Err(_) => (stories, serde_json::json!([])),
        };
        let body = serde_json::json!({
            "stories": ordered,
            "icebox_unified": icebox_unified,
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
            // #333: apply the persisted priority index so drag-reorder
            // survives refetch. Without this, the frontend's
            // optimistic reorder gets overwritten on the very next
            // /api/board call with GitHub's native order.
            let (ordered, icebox_unified) = match crate::session::priority::default_priority_path()
            {
                Ok(path) => {
                    let index = crate::session::priority::load_priority(&path);
                    let icebox_unified = serde_json::to_value(&index.icebox_unified)
                        .unwrap_or_else(|_| serde_json::json!([]));
                    (
                        crate::session::priority::apply_priority_to_board(stories, &index),
                        icebox_unified,
                    )
                }
                Err(_) => (stories, serde_json::json!([])),
            };
            let body = serde_json::json!({
                "stories": ordered,
                "icebox_unified": icebox_unified,
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
/// `session::pins`. Writes dispatch `PinsAction::PinAdded`/`PinRemoved`
/// to the orchestrator, which fans out a typed `StateDelta::Pins`
/// over WS so peer mirror.pins updates within WS latency.
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
    axum::Extension(orchestrator): axum::Extension<
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    >,
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
    orchestrator
        .dispatch(crate::orchestrator::OrchestratorAction::Reactions(
            crate::orchestrator::ReactionsAction::ReactionToggled {
                channel: body.channel.clone(),
                message_key: body.message_key.clone(),
                emoji: body.emoji.clone(),
                user: body.user.clone(),
                ts_iso: now.clone(),
            },
        ))
        .await;
    let excerpt = session
        .read_messages()
        .ok()
        .and_then(|msgs| find_message_excerpt(&msgs, &body.message_key, EXCERPT_MAX_LEN));
    broadcast_reaction_event(&session, &body, present, excerpt.as_deref());
    (
        StatusCode::OK,
        json_headers(),
        serde_json::json!({ "present": present }).to_string(),
    )
}

#[derive(Deserialize)]
pub struct IterationMarkerCreate {
    #[serde(default)]
    pub theme_name: Option<String>,
}

pub async fn get_iteration_markers(
    State(session): State<Arc<session::Session>>,
) -> impl IntoResponse {
    let store = session::iteration_markers::load(&session.dir);
    let markers = session::iteration_markers::list(&store);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&markers).unwrap_or_else(|_| "[]".to_string()),
    )
}

pub async fn post_iteration_marker(
    State(session): State<Arc<session::Session>>,
    axum::Json(body): axum::Json<IterationMarkerCreate>,
) -> impl IntoResponse {
    let mut store = session::iteration_markers::load(&session.dir);
    let theme = body
        .theme_name
        .unwrap_or_else(|| session::iteration_markers::next_unused_theme(&store));
    let now = chrono::Utc::now().to_rfc3339();
    let marker = session::iteration_markers::add(&mut store, theme, now);
    if let Err(e) = session::iteration_markers::save(&session.dir, &store) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(&format!("save iteration_markers: {e}")),
        );
    }
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&marker).unwrap_or_else(|_| "{}".to_string()),
    )
}

#[derive(Deserialize, Default)]
pub struct CloseIterationParams {
    /// AC 16-17 carry_over flag (Q1 PM-ratified YAGNI v1 boolean).
    /// When true, unfinished ready-stories remain in ready instead of
    /// dropping back to icebox-top. Default false (drop-back).
    #[serde(default)]
    pub carry_over: bool,
}

/// #1258 close_iteration α-substrate: REST endpoint that runs the
/// cross-store atomic step (promote stories above topmost marker,
/// delete topmost marker, drop unfinished ready-stories to icebox-top).
/// Returns the outcome as JSON. Bridge handler POSTs here to keep
/// state mutation behind middleware (per `feedback_middleware_owns_external_io`)
/// and ensures REST + MCP share the canonical mutation chain (per
/// `feedback_rest_mcp_canonical_mutation_parity`).
///
/// Best-effort cross-store atomicity (AC 20-22): pre-mutation
/// snapshots of priority + markers are captured; on save-failure the
/// snapshots are restored to disk, leaving pre-fire state intact.
pub async fn post_close_iteration(
    State(session): State<Arc<session::Session>>,
    Query(params): Query<CloseIterationParams>,
) -> impl IntoResponse {
    let priority_path = match session::priority::default_priority_path() {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("priority path: {e}")),
            );
        }
    };
    let priority_snapshot = session::priority::load_priority(&priority_path);
    let markers_snapshot = session::iteration_markers::load(&session.dir);
    let mut priority = priority_snapshot.clone();
    let mut markers_store = markers_snapshot.clone();
    let unfinished_ready: Vec<u64> = priority.columns.get("ready").cloned().unwrap_or_default();
    let outcome = session::priority::close_iteration_atomic(
        &mut priority,
        &mut markers_store.markers,
        &unfinished_ready,
        params.carry_over,
    );
    if let Err(e) = session::priority::save_priority(&priority_path, &priority) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(&format!("save priority: {e}")),
        );
    }
    if let Err(e) = session::iteration_markers::save(&session.dir, &markers_store) {
        // AC 22: rollback priority to pre-fire state when markers save fails.
        if let Err(rb) = session::priority::save_priority(&priority_path, &priority_snapshot) {
            session::diag::log_diag(&format!(
                "post_close_iteration: rollback after markers save-failure failed: {rb}"
            ));
        }
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(&format!("save iteration_markers: {e}")),
        );
    }
    let payload = session::message::StateDeltaPayload::IterationClose {
        promoted: outcome.promoted.clone(),
        dropped_back: outcome.dropped_back.clone(),
        deleted_marker_id: outcome.deleted_marker_id,
    };
    let delta_msg = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(payload),
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&delta_msg) {
        session::diag::log_diag(&format!(
            "post_close_iteration: state-delta broadcast failed: {e}"
        ));
    }
    let body = serde_json::json!({
        "promoted": outcome.promoted,
        "dropped_back": outcome.dropped_back,
        "deleted_marker_id": outcome.deleted_marker_id,
    });
    (StatusCode::OK, json_headers(), body.to_string())
}

/// #387 cross-agent reaction visibility. Typed `StateDelta::Reactions`
/// frames are renderer-mirror-only; MCP-bridged agents see chat
/// insights rather than wire-state. This synthesizes a human-readable
/// insight-class chat entry so agents see "{user} reacted {emoji} to
/// {sender}'s message" (or "removed {emoji} from…" on a toggle-off)
/// in their stream.
///
/// Message-key shape is `{ts}|{from}` (mirrors session::pins /
/// usePins / pinKey); we parse the `from` out to name the original
/// sender by short-role. If parsing fails, the synthesized text
/// still carries user + emoji; agents lose only the "whose message"
/// context.
/// Maximum excerpt length before truncation kicks in. Designer-spec'd
/// ~40 chars; slight padding for readable word-boundary snips.
pub(crate) const EXCERPT_MAX_LEN: usize = 40;

/// #400: truncate a message body to a readable excerpt for the
/// reaction event text. Returns the body unchanged when ≤ max_len
/// (designer 12:08 UTC: skip the ellipsis when the full text fits).
/// Otherwise truncates on the last whitespace boundary ≤ max_len
/// when possible, falling back to hard-char truncation if no
/// boundary is in range. Replaces newlines with spaces to keep
/// the synthesized text single-line.
pub(crate) fn truncate_excerpt(text: &str, max_len: usize) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let flat = flat.trim();
    if flat.chars().count() <= max_len {
        return flat.to_string();
    }
    // Walk to the char boundary at max_len + rewind to last
    // whitespace if any exists in that window. char_indices gives
    // us a safe byte cut point.
    let mut last_space: Option<usize> = None;
    let mut cut: usize = flat.len();
    for (i, (byte_idx, ch)) in flat.char_indices().enumerate() {
        if i == max_len {
            cut = byte_idx;
            break;
        }
        if ch.is_whitespace() {
            last_space = Some(byte_idx);
        }
    }
    let snip_end = last_space.unwrap_or(cut);
    let mut out = flat[..snip_end].trim_end().to_string();
    out.push('…');
    out
}

/// #400 lookup: find the target message's text by the reaction
/// toggle's message_key (`{ts}|{from}`). Returns Some(excerpt) on
/// match, None on parse failure / message-not-found / empty body.
/// The excerpt is already truncated per `truncate_excerpt`.
pub(crate) fn find_message_excerpt(
    messages: &[session::Message],
    message_key: &str,
    max_len: usize,
) -> Option<String> {
    let (ts_str, from) = message_key.split_once('|')?;
    if ts_str.is_empty() || from.is_empty() {
        return None;
    }
    for m in messages {
        if m.from != from {
            continue;
        }
        if m.ts.to_rfc3339() != ts_str {
            continue;
        }
        if m.text.is_empty() {
            return None;
        }
        return Some(truncate_excerpt(&m.text, max_len));
    }
    None
}

/// Core synthesis; #400 threads the optional excerpt through. The
/// no-excerpt path matches #387's original shape exactly so the
/// fallback behavior when lookup fails is byte-identical to
/// pre-#400 output.
pub(crate) fn build_reaction_event_text_with_excerpt(
    body: &ReactionToggle,
    present: bool,
    excerpt: Option<&str>,
) -> String {
    let original_sender_short = body
        .message_key
        .split_once('|')
        .map(|(_ts, from)| from.split(':').next().unwrap_or(from).to_string());
    // Designer 2026-04-23 11:58 UTC: "removed {emoji}" reads as
    // ambiguous ("{user} removed my reaction"). "unreacted" keeps
    // the "who did what to what" structure clear — the user is
    // removing their own reaction, not someone else's.
    let base = match (present, original_sender_short) {
        (true, Some(sender)) if !sender.is_empty() => {
            format!(
                "{} reacted {} to {}'s message",
                body.user, body.emoji, sender
            )
        }
        (true, _) => format!("{} reacted {}", body.user, body.emoji),
        (false, Some(sender)) if !sender.is_empty() => {
            format!(
                "{} unreacted {} from {}'s message",
                body.user, body.emoji, sender
            )
        }
        (false, _) => format!("{} unreacted {}", body.user, body.emoji),
    };
    // #400: wrap the excerpt in typographic quotes to avoid
    // nested-quote ambiguity with any " chars inside the body
    // text. Falling through without an excerpt keeps #387's
    // shape verbatim.
    match excerpt {
        Some(e) if !e.is_empty() => format!("{base}: “{e}”"),
        _ => base,
    }
}

fn broadcast_reaction_event(
    session: &session::Session,
    body: &ReactionToggle,
    present: bool,
    excerpt: Option<&str>,
) {
    let text = build_reaction_event_text_with_excerpt(body, present, excerpt);
    let msg = session::Message {
        from: "bpmux".to_string(),
        text,
        ts: chrono::Utc::now(),
        message_type: session::MessageType::Insight,
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!("rest: reaction-event broadcast failed: {e}"));
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
    if payload.state == "icebox" {
        // #1258 α-substrate: icebox reorder operates on the unified
        // positional space (markers + stories share the same list).
        crate::session::priority::reorder_icebox_unified(
            &mut index,
            crate::session::priority::IceboxItem::Story { number },
            payload.position,
        );
    } else {
        crate::session::priority::reorder(&mut index, &payload.state, number, payload.position);
    }
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

/// #1258 α-substrate: POST /api/iteration_markers/:id/reorder — move
/// a marker to a position in `icebox_unified`. Markers + stories share
/// the same positional space; this lets PM/owner drag markers across
/// stories to redefine iteration boundaries.
pub async fn post_reorder_iteration_marker(
    State(session): State<Arc<session::Session>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(payload): Json<ReorderMarkerPayload>,
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
    crate::session::priority::reorder_icebox_unified(
        &mut index,
        crate::session::priority::IceboxItem::Marker { id },
        payload.position,
    );
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

#[derive(Deserialize)]
pub struct ReorderMarkerPayload {
    pub position: usize,
}
