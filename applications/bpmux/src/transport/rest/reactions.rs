//! Reactions REST (#340) — `GET/POST /api/reactions`.
//!
//! POST toggles: if the (user, emoji, message_key) triple exists,
//! it's removed; otherwise added. Single endpoint for add + remove
//! keeps the renderer's click-handler path trivial.
//!
//! #387 cross-agent reaction visibility: typed `StateDelta::Reactions`
//! frames are renderer-mirror-only; MCP-bridged agents see chat
//! insights rather than wire-state. The post handler synthesizes a
//! human-readable insight-class chat entry so agents see
//! `"{user} reacted {emoji} to {sender}'s message"` (or the
//! `unreacted` variant on toggle-off) in their stream.
//!
//! Message-key shape is `{ts}|{from}` (mirrors `session::pins` /
//! `usePins` / `pinKey`); the synthesis parses `from` to name the
//! original sender by short-role. If parsing fails the synthesized
//! text still carries `{user}` + `{emoji}`; agents lose only the
//! "whose message" context.

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

/// Maximum excerpt length before truncation kicks in. Designer-spec'd
/// ~40 chars; slight padding for readable word-boundary snips.
pub(crate) const EXCERPT_MAX_LEN: usize = 40;

/// #400: truncate a message body to a readable excerpt for the
/// reaction event text. Returns the body unchanged when ≤ max_len
/// (designer 12:08 UTC: skip the ellipsis when the full text fits).
/// Otherwise walks to the char boundary at `max_len` then rewinds
/// to the last whitespace boundary in that window (via
/// `char_indices` for a safe byte cut); falls back to hard-char
/// truncation if no whitespace is in range. Replaces newlines with
/// spaces to keep the synthesized text single-line.
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

/// Core reaction-event-text synthesis; #400 threads the optional
/// excerpt through. The no-excerpt path matches #387's original
/// shape exactly so the fallback behavior when lookup fails is
/// byte-identical to pre-#400 output.
///
/// Designer 2026-04-23 11:58 UTC: "removed {emoji}" reads ambiguously
/// ("{user} removed my reaction"). "unreacted" keeps the
/// who-did-what-to-what structure clear — the user is removing their
/// own reaction, not someone else's.
///
/// When `excerpt` is non-empty, the body wraps it in typographic
/// quotes (`“…”`) to avoid nested-quote ambiguity with `"` chars
/// inside the body text. Falling through without an excerpt keeps
/// #387's shape verbatim.
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
