use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::session;

pub fn seed_api_enabled() -> bool {
    std::env::var("BPMUX_TEST_SEED_API")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[derive(Deserialize)]
pub struct SeedMessagesBody {
    pub messages: Vec<session::Message>,
}

pub async fn post_seed_messages(
    State(session): State<Arc<session::Session>>,
    Json(body): Json<SeedMessagesBody>,
) -> impl IntoResponse {
    for msg in &body.messages {
        let mut msg = msg.clone();
        msg.ensure_id();
        if let Err(e) = session.storage.append_message(&msg) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("seed_messages append failed: {e}"),
            );
        }
    }
    (
        StatusCode::CREATED,
        format!("seeded {} messages", body.messages.len()),
    )
}

#[derive(Deserialize)]
pub struct SeedReactionsBody {
    pub store: session::reactions::ReactionsStore,
}

pub async fn post_seed_reactions(
    State(session): State<Arc<session::Session>>,
    Json(body): Json<SeedReactionsBody>,
) -> impl IntoResponse {
    if let Err(e) = session.storage.save_reactions(&body.store) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("seed_reactions save failed: {e}"),
        );
    }
    (StatusCode::CREATED, "seeded reactions".to_string())
}

pub async fn post_reset_messages(
    State(session): State<Arc<session::Session>>,
) -> impl IntoResponse {
    if let Err(e) = session.storage.clear_messages() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("reset_messages failed: {e}"),
        );
    }
    (StatusCode::OK, "reset messages".to_string())
}

pub async fn post_reset_stories(
    State(session): State<Arc<session::Session>>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
) -> impl IntoResponse {
    let Some(refresh) = test_refresh.as_ref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "reset_stories: tracker not configured (test-refresh handle absent)".to_string(),
        );
    };
    refresh.store().clear();
    let invalidate_delta = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(session::message::StateDeltaPayload::BoardInvalidated {
            reason: "board reset".to_string(),
        }),
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&invalidate_delta) {
        crate::diag::log_diag(&format!(
            "post_reset_stories: board_invalidated state-delta broadcast failed: {e}"
        ));
    }
    crate::transport::rest::shared::broadcast_board_snapshot(&session, refresh);
    (StatusCode::OK, "reset stories".to_string())
}

#[derive(Deserialize)]
pub struct SeedRosterBody {
    pub identities: Vec<String>,
    #[serde(default)]
    pub presence: Option<crate::orchestrator::Presence>,
}

#[derive(Deserialize)]
pub struct SeedStory {
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default = "default_state_icebox")]
    pub initial_state: String,
    #[serde(default = "default_story_type_feature")]
    pub story_type: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub is_closed: bool,
}

fn default_state_icebox() -> String {
    "icebox".to_string()
}

fn default_story_type_feature() -> String {
    "feature".to_string()
}

#[derive(Deserialize)]
pub struct SeedStoriesBody {
    pub stories: Vec<SeedStory>,
}

pub async fn post_seed_stories(
    State(session): State<Arc<session::Session>>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    Json(body): Json<SeedStoriesBody>,
) -> impl IntoResponse {
    let Some(refresh) = test_refresh.as_ref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "seed_stories: tracker not configured (test-refresh handle absent)".to_string(),
        );
    };
    for s in &body.stories {
        let initial_state = match s.initial_state.as_str() {
            "icebox" => crate::board::domain::StoryState::Icebox,
            "ready" => crate::board::domain::StoryState::Ready,
            "in-progress" => crate::board::domain::StoryState::InProgress,
            "desk-check" => crate::board::domain::StoryState::DeskCheck,
            "delivered" => crate::board::domain::StoryState::Delivered,
            "accepted" => crate::board::domain::StoryState::Accepted,
            _ => crate::board::domain::StoryState::Icebox,
        };
        let story_type = match s.story_type.as_str() {
            "iteration" => crate::board::domain::StoryType::Iteration,
            "epic" => crate::board::domain::StoryType::Epic,
            "bug" => crate::board::domain::StoryType::Bug,
            "chore" => crate::board::domain::StoryType::Chore,
            _ => crate::board::domain::StoryType::Feature,
        };
        let meta = crate::board::events::EventMeta {
            actor: "test-seed".to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        if let Err(e) = refresh.mutate_sync(crate::board::BoardEvent::StoryCreated {
            meta,
            number: s.number,
            title: s.title.clone(),
            body: s.body.clone(),
            initial_state,
            story_type,
        }) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("seed_stories: StoryCreated #{} failed: {e}", s.number),
            );
        }
        for label in &s.labels {
            let tag_meta = crate::board::events::EventMeta {
                actor: "test-seed".to_string(),
                ts: chrono::Utc::now(),
                peer_id: crate::board::PeerId::local(),
            };
            if let Err(e) = refresh.mutate_sync(crate::board::BoardEvent::TagAdded {
                meta: tag_meta,
                number: s.number,
                label: label.clone(),
            }) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!(
                        "seed_stories: TagAdded {label} on #{} failed: {e}",
                        s.number
                    ),
                );
            }
        }
        if s.is_closed {
            let close_meta = crate::board::events::EventMeta {
                actor: "test-seed".to_string(),
                ts: chrono::Utc::now(),
                peer_id: crate::board::PeerId::local(),
            };
            if let Err(e) = refresh.mutate_sync(crate::board::BoardEvent::StoryClosed {
                meta: close_meta,
                number: s.number,
                reason: Some("test-seed: closed at seed time".to_string()),
            }) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("seed_stories: StoryClosed #{} failed: {e}", s.number),
                );
            }
        }
    }
    let invalidate_delta = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(session::message::StateDeltaPayload::BoardInvalidated {
            reason: "board changed".to_string(),
        }),
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&invalidate_delta) {
        crate::diag::log_diag(&format!(
            "post_seed_stories: board_invalidated state-delta broadcast failed: {e}"
        ));
    }
    crate::transport::rest::shared::broadcast_board_snapshot(&session, refresh);
    (
        StatusCode::CREATED,
        format!("seeded {} stories", body.stories.len()),
    )
}

pub async fn post_seed_roster(
    axum::Extension(orchestrator): axum::Extension<
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    >,
    Json(body): Json<SeedRosterBody>,
) -> impl IntoResponse {
    for identity in &body.identities {
        let name = identity
            .split(':')
            .next()
            .unwrap_or(identity.as_str())
            .to_string();
        let role = crate::transport::ws::role_from_short_name(&name);
        orchestrator
            .dispatch(crate::orchestrator::OrchestratorAction::Roster(
                crate::orchestrator::RosterAction::RosterMemberUpserted {
                    routing_id: identity.clone(),
                    name,
                    identity: identity.clone(),
                    role,
                    presence: body
                        .presence
                        .unwrap_or(crate::orchestrator::Presence::Online),
                },
            ))
            .await;
    }
    (
        StatusCode::OK,
        format!("seeded {} roster identities", body.identities.len()),
    )
}
