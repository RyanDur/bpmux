use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::{err_body, json_headers};
use crate::session;

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

pub(crate) const EXCERPT_MAX_LEN: usize = 40;

pub(crate) fn truncate_excerpt(text: &str, max_len: usize) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let flat = flat.trim();
    if flat.chars().count() <= max_len {
        return flat.to_string();
    }
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

pub(crate) fn build_reaction_event_text_with_excerpt(
    body: &ReactionToggle,
    present: bool,
    excerpt: Option<&str>,
) -> String {
    let original_sender_short = body
        .message_key
        .split_once('|')
        .map(|(_ts, from)| from.split(':').next().unwrap_or(from).to_string());
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
