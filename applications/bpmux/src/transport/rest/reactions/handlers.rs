use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::excerpt::{
    EXCERPT_MAX_LEN, build_reaction_event_text_with_excerpt, find_message_excerpt,
    message_scope_from_key,
};
use super::{ReactionToggle, ReactionsChannelQuery};
use crate::session;
use crate::transport::rest::shared::{err_body, json_headers};

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
    match apply_reaction_toggle(&session, &orchestrator, &body).await {
        Ok(present) => (
            StatusCode::OK,
            json_headers(),
            serde_json::json!({ "present": present }).to_string(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(&e),
        ),
    }
}

pub async fn apply_reaction_toggle(
    session: &session::Session,
    orchestrator: &Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    body: &ReactionToggle,
) -> Result<bool, String> {
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
    session::reactions::save(&session.dir, &store).map_err(|e| format!("save reactions: {e}"))?;
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
    const EXCERPT_SCAN_WINDOW: usize = 2000;
    let msgs = session.read_recent(EXCERPT_SCAN_WINDOW, 0, &|_| true).ok();
    let excerpt = msgs
        .as_deref()
        .and_then(|m| find_message_excerpt(m, &body.message_key, EXCERPT_MAX_LEN));
    let scope = msgs
        .as_deref()
        .and_then(|m| message_scope_from_key(m, &body.message_key));
    broadcast_reaction_event(session, body, present, excerpt.as_deref(), scope);
    Ok(present)
}

fn broadcast_reaction_event(
    session: &session::Session,
    body: &ReactionToggle,
    present: bool,
    excerpt: Option<&str>,
    scope: Option<Vec<String>>,
) {
    let text = build_reaction_event_text_with_excerpt(body, present, excerpt);
    let msg = session::Message {
        from: "bpmux".to_string(),
        text,
        ts: chrono::Utc::now(),
        message_type: session::MessageType::Insight,
        to: scope,
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        crate::diag::log_diag(&format!("rest: reaction-event broadcast failed: {e}"));
    }
}
