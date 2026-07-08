use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::product_management::label_codec::LabelCodec;
use crate::session;

pub fn seed_api_enabled() -> bool {
    std::env::var("BPMUX_TEST_SEED_API")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[derive(Deserialize)]
pub struct SeedMessagesBody {
    pub messages: Vec<crate::message::Message>,
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
    pub store: crate::reactions::ReactionsStore,
}

pub async fn post_seed_reactions(
    State(session): State<Arc<session::Session>>,
    Json(body): Json<SeedReactionsBody>,
) -> impl IntoResponse {
    if let Err(e) = crate::reactions::save(&session.dir, &body.store) {
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
    axum::Extension(orchestrator): axum::Extension<crate::sync::OrchestratorHandle>,
) -> impl IntoResponse {
    let Some(refresh) = orchestrator.as_ref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "reset_stories: tracker not configured (test-refresh handle absent)".to_string(),
        );
    };
    refresh.clear_stories();
    let invalidate_delta = crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(crate::message::StateDeltaPayload::BoardInvalidated {
            reason: "board reset".to_string(),
        }),
        ..crate::message::Message::default()
    };
    if let Err(e) = session.broadcast(&invalidate_delta) {
        crate::diag::log_diag(&format!(
            "post_reset_stories: board_invalidated state-delta broadcast failed: {e}"
        ));
    }
    crate::transport::rest::shared::broadcast_stories(&session, refresh);
    (StatusCode::OK, "reset stories".to_string())
}

#[derive(Deserialize)]
pub struct SeedRosterBody {
    pub identities: Vec<String>,
    #[serde(default)]
    pub presence: Option<crate::store::Presence>,
}

#[derive(Deserialize)]
pub struct SeedStory {
    pub id: u64,
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
    axum::Extension(orchestrator): axum::Extension<crate::sync::OrchestratorHandle>,
    axum::Extension(dispatcher): axum::Extension<crate::store::Dispatch>,
    Json(body): Json<SeedStoriesBody>,
) -> impl IntoResponse {
    let Some(refresh) = orchestrator.as_ref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "seed_stories: tracker not configured (test-refresh handle absent)".to_string(),
        );
    };
    for s in &body.stories {
        let initial_state = match s.initial_state.as_str() {
            "icebox" | "ready" => crate::product_management::story::Work::NotStarted,
            "in-progress" | "desk-check" => crate::product_management::story::Work::InProgress,
            "delivered" | "acceptance" => crate::product_management::story::Work::Acceptance,
            "accepted" => crate::product_management::story::Work::Accepted,
            _ => crate::product_management::story::Work::NotStarted,
        };
        let is_iteration_seed = s.story_type.as_str() == "iteration";
        let is_epic_seed = s.story_type.as_str() == "epic";
        let story_type = {
            use crate::product_management::story::StoryKind;
            match s.story_type.as_str() {
                "bug" => StoryKind::Bug,
                "chore" => StoryKind::Chore,
                _ => StoryKind::Feature,
            }
        };
        let meta = crate::product_management::story::events::EventMeta {
            actor: "test-seed".to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::story::peer::PeerId::local(),
        };
        if let Err(e) =
            refresh.mutate_sync(crate::product_management::story::events::StoryEvent::new(
                meta,
                crate::product_management::story::StoryId::new(s.id),
                crate::product_management::story::events::StoryEventKind::StoryCreated {
                    title: s.title.clone(),
                    body: s.body.clone(),
                    initial_state: initial_state.clone(),
                    story_type,
                },
            ))
        {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("seed_stories: StoryCreated #{} failed: {e}", s.id),
            );
        }
        if s.initial_state == "desk-check" {
            dispatcher.dispatch(crate::store::AppAction::Board(
                crate::board::BoardAction::StoryPlaced {
                    story: crate::product_management::story::StoryId::new(s.id),
                    stage: crate::product_management::story::Stage::DeskCheck.id(),
                },
            ));
        }
        let type_label_meta = crate::product_management::story::events::EventMeta {
            actor: "test-seed".to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::story::peer::PeerId::local(),
        };
        let type_label = if is_iteration_seed {
            "iteration".to_string()
        } else if is_epic_seed {
            "epic".to_string()
        } else {
            crate::integrations::github::gh_label_codec::GhLabelCodec.encode_story_type(story_type)
        };
        if let Err(e) =
            refresh.mutate_sync(crate::product_management::story::events::StoryEvent::new(
                type_label_meta,
                crate::product_management::story::StoryId::new(s.id),
                crate::product_management::story::events::StoryEventKind::TagAdded {
                    label: type_label,
                },
            ))
        {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("seed_stories: type-label TagAdded on #{} failed: {e}", s.id),
            );
        }
        for label in &s.labels {
            let tag_meta = crate::product_management::story::events::EventMeta {
                actor: "test-seed".to_string(),
                ts: chrono::Utc::now(),
                peer_id: crate::product_management::story::peer::PeerId::local(),
            };
            let codec = crate::integrations::github::gh_label_codec::GhLabelCodec;
            if codec.is_iteration_label_loose(label) {
                continue;
            }
            let event = crate::product_management::story::events::StoryEvent::new(
                tag_meta,
                crate::product_management::story::StoryId::new(s.id),
                crate::product_management::story::events::StoryEventKind::TagAdded {
                    label: label.clone(),
                },
            );
            if let Err(e) = refresh.mutate_sync(event) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("seed_stories: label event {label} on #{} failed: {e}", s.id),
                );
            }
        }
        if s.is_closed {
            let close_meta = crate::product_management::story::events::EventMeta {
                actor: "test-seed".to_string(),
                ts: chrono::Utc::now(),
                peer_id: crate::product_management::story::peer::PeerId::local(),
            };
            if let Err(e) =
                refresh.mutate_sync(crate::product_management::story::events::StoryEvent::new(
                    close_meta,
                    crate::product_management::story::StoryId::new(s.id),
                    crate::product_management::story::events::StoryEventKind::StoryClosed {
                        reason: Some("test-seed: closed at seed time".to_string()),
                    },
                ))
            {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("seed_stories: StoryClosed #{} failed: {e}", s.id),
                );
            }
        }
        if is_iteration_seed || is_epic_seed {
            let mut carrier = crate::product_management::story::Story::new_minimal(
                crate::product_management::story::StoryId::new(s.id),
                s.title.clone(),
                initial_state,
                chrono::Utc::now(),
                "test-seed".to_string(),
            );
            carrier.body = Some(s.body.clone());
            if is_iteration_seed {
                dispatcher.dispatch(crate::store::AppAction::Iteration(
                    crate::product_management::iteration::IterationAction::Upserted(vec![
                        crate::integrations::github::mapper::decode_iteration_from_story(
                            &carrier,
                            &s.labels,
                            s.is_closed,
                        ),
                    ]),
                ));
            } else {
                dispatcher.dispatch(crate::store::AppAction::Epic(
                    crate::product_management::epic::EpicAction::Upserted(vec![
                        crate::integrations::github::mapper::decode_epic_from_story(
                            &carrier,
                            s.is_closed,
                        ),
                    ]),
                ));
            }
        }
    }
    let invalidate_delta = crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(crate::message::StateDeltaPayload::BoardInvalidated {
            reason: "board changed".to_string(),
        }),
        ..crate::message::Message::default()
    };
    if let Err(e) = session.broadcast(&invalidate_delta) {
        crate::diag::log_diag(&format!(
            "post_seed_stories: board_invalidated state-delta broadcast failed: {e}"
        ));
    }
    crate::transport::rest::shared::broadcast_stories(&session, refresh);
    (
        StatusCode::CREATED,
        format!("seeded {} stories", body.stories.len()),
    )
}

pub async fn post_seed_roster(
    axum::Extension(store): axum::Extension<Arc<crate::store::Store>>,
    Json(body): Json<SeedRosterBody>,
) -> impl IntoResponse {
    for identity in &body.identities {
        let name = identity
            .split(':')
            .next()
            .unwrap_or(identity.as_str())
            .to_string();
        let role = crate::transport::ws::role_from_short_name(&name);
        store
            .dispatch(crate::store::AppAction::Roster(
                crate::store::RosterAction::RosterMemberUpserted {
                    routing_id: identity.clone(),
                    name,
                    identity: identity.clone(),
                    role,
                    presence: body.presence.unwrap_or(crate::store::Presence::Online),
                },
            ))
            .await;
    }
    (
        StatusCode::OK,
        format!("seeded {} roster identities", body.identities.len()),
    )
}
