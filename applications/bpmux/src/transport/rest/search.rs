//! Cross-substrate search — messages + story bodies + story comments
//! collapsed into one HTTP RTT for the renderer (#344 AC7). When the
//! tracker is unconfigured, story/comment buckets return empty arrays
//! instead of 503 so the renderer renders "0 results" uniformly.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::TrackerHandle;
use crate::session;

#[derive(Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub sender: Option<String>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub channel: Option<String>,
    pub message_type: Option<String>,
}

/// GET /api/search — single-RTT search across messages, story
/// bodies, and story comments (#344 AC7). Stories + comments need
/// the tracker; when `TrackerHandle` is None (GH_TOKEN absent) those
/// buckets return empty arrays so the renderer shows "0 results"
/// without surfacing a 503 distinct from the messages-only path.
pub async fn search_messages(
    State(session): State<Arc<session::Session>>,
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
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
/// `(story_hits, comment_hits)`. Empty vecs when tracker is None —
/// caller doesn't surface a different error for missing config.
///
/// Story hits substring-match on title OR body; the snippet
/// extracts ~80 chars around the first body match so the renderer
/// has a preview line without needing to fetch the full body.
///
/// Comment hits fan out N `tracker.list_story_comments` calls (one
/// per story) concurrently via `join_all`, collapsing N×RTT into
/// one RTT-group. Manageable at bpmux scale (~20-50 stories); above
/// that we'd want a server-side search API on the tracker instead
/// of client-side filtering.
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
