use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::TrackerHandle;

#[derive(Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub sender: Option<String>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub addressee: Option<String>,
    pub message_type: Option<String>,
}

pub async fn search_messages(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(tracker_cell): axum::Extension<super::shared::TrackerCell>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let tracker: TrackerHandle = tracker_cell.current();
    let message_class = params.message_type.as_deref().and_then(|t| match t {
        "correction" | "experience" => Some(comprehension::MessageClass::Experience(
            comprehension::classifier::ExperienceKind::Correction,
        )),
        "decision" => Some(comprehension::MessageClass::Decision),
        "discussion" => Some(comprehension::MessageClass::Discussion),
        _ => None,
    });
    let filters = comprehension::SearchFilters {
        query: params.query.clone(),
        sender: params.sender.clone(),
        after: params.after.as_ref().and_then(|s| s.parse().ok()),
        before: params.before.as_ref().and_then(|s| s.parse().ok()),
        addressee: params.addressee.clone(),
        message_class,
        insight_only: params.message_type.as_deref() == Some("insight"),
        owner_name: hub.owner_name.clone(),
    };
    let query_lower = filters.query.to_lowercase();
    let radius = comprehension::CONTEXT_WINDOW as u64;
    let mut accepted = Vec::new();
    let messages_part = match hub.search_conversations(
        &query_lower,
        filters.addressee.as_deref(),
        radius,
        &mut |cluster, center| match comprehension::evaluate_candidate(cluster, center, &filters) {
            Some(result) => {
                accepted.push(result);
                true
            }
            None => false,
        },
    ) {
        Ok(()) => accepted,
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
            "id": s.get("number"),
            "title": title,
            "snippet": snippet,
            "matched_in": if title_match { "title" } else { "body" },
        }));
    }

    let story_ids: Vec<u64> = stories
        .iter()
        .filter_map(|s| s.get("number").and_then(|n| n.as_u64()))
        .collect();
    let futures = story_ids
        .iter()
        .map(|n| async move { (*n, tracker.list_story_comments(*n).await) });
    let results = futures::future::join_all(futures).await;
    let mut comment_hits: Vec<serde_json::Value> = Vec::new();
    for (story_id, res) in results {
        let Ok(comments) = res else { continue };
        for c in comments {
            let body = c.get("body").and_then(|v| v.as_str()).unwrap_or("");
            if !body.to_lowercase().contains(&needle) {
                continue;
            }
            comment_hits.push(serde_json::json!({
                "story_id": story_id,
                "comment_id": c.get("id"),
                "author": c.get("author"),
                "snippet": extract_snippet(body, &needle),
            }));
        }
    }

    (story_hits, comment_hits)
}

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
