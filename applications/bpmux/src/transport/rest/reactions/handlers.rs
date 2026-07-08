use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::excerpt::{
    EXCERPT_MAX_LEN, build_reaction_event_text_with_excerpt, find_message_excerpt,
    message_scope_from_key,
};
use super::{ReactionToggle, ReactionsChannelQuery};
use crate::hub;
use crate::transport::rest::shared::{err_body, json_headers};

pub async fn get_reactions(
    axum::Extension(store): axum::Extension<Arc<crate::store::Store>>,
    Query(params): Query<ReactionsChannelQuery>,
) -> impl IntoResponse {
    let agg = store.reactions_aggregated_for_channel(&params.channel);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&agg).unwrap_or_else(|_| "[]".to_string()),
    )
}

pub async fn post_reaction(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(store): axum::Extension<Arc<crate::store::Store>>,
    axum::Json(body): axum::Json<ReactionToggle>,
) -> impl IntoResponse {
    match apply_reaction_toggle(&hub, &store, &body).await {
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
    hub: &hub::Hub,
    store: &Arc<crate::store::Store>,
    body: &ReactionToggle,
) -> Result<bool, String> {
    let now = chrono::Utc::now().to_rfc3339();
    store
        .dispatch(crate::store::AppAction::Reactions(
            crate::store::ReactionsAction::ReactionToggled {
                channel: body.channel.clone(),
                message_key: body.message_key.clone(),
                emoji: body.emoji.clone(),
                user: body.user.clone(),
                ts_iso: now.clone(),
            },
        ))
        .await;
    let present =
        store.reactions_is_present(&body.channel, &body.message_key, &body.emoji, &body.user);
    const EXCERPT_SCAN_WINDOW: usize = 2000;
    let msgs = hub.read_recent(EXCERPT_SCAN_WINDOW, 0, &|_| true).ok();
    let excerpt = msgs
        .as_deref()
        .and_then(|m| find_message_excerpt(m, &body.message_key, EXCERPT_MAX_LEN));
    let scope = msgs
        .as_deref()
        .and_then(|m| message_scope_from_key(m, &body.message_key));
    broadcast_reaction_event(hub, body, present, excerpt.as_deref(), scope);
    Ok(present)
}

fn broadcast_reaction_event(
    hub: &hub::Hub,
    body: &ReactionToggle,
    present: bool,
    excerpt: Option<&str>,
    scope: Option<Vec<String>>,
) {
    let text = build_reaction_event_text_with_excerpt(body, present, excerpt);
    let msg = crate::message::Message {
        from: "bpmux".to_string(),
        text,
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::Insight,
        to: scope,
        ..crate::message::Message::default()
    };
    if let Err(e) = hub.broadcast(&msg) {
        crate::diag::log_diag(&format!("rest: reaction-event broadcast failed: {e}"));
    }
}
