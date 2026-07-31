use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use product_management::label_codec::LabelCodec;

pub fn seed_api_enabled() -> bool {
    std::env::var("BPMUX_TEST_SEED_API")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[derive(Deserialize)]
pub struct SeedMessagesBody {
    pub messages: Vec<message::Message>,
}

pub async fn post_seed_messages(
    State(hub): State<Arc<hub::Hub>>,
    Json(body): Json<SeedMessagesBody>,
) -> impl IntoResponse {
    for msg in &body.messages {
        let mut msg = msg.clone();
        msg.ensure_id();
        if let Err(e) = hub.conversation.append_message(&msg) {
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
    pub store: reactions::ReactionsStore,
}

pub async fn post_seed_reactions(
    axum::Extension(store): axum::Extension<Arc<store::Store>>,
    Json(body): Json<SeedReactionsBody>,
) -> impl IntoResponse {
    for (channel, list) in &body.store.channels {
        for reaction in list {
            store
                .dispatch(store::AppAction::Reactions(
                    store::ReactionsAction::ReactionToggled {
                        channel: channel.clone(),
                        message_key: reaction.message_key.clone(),
                        emoji: reaction.emoji.clone(),
                        user: reaction.user.clone(),
                        ts_iso: reaction.reacted_at.clone(),
                    },
                ))
                .await;
        }
    }
    (StatusCode::CREATED, "seeded reactions".to_string())
}

pub async fn post_reset_messages(State(hub): State<Arc<hub::Hub>>) -> impl IntoResponse {
    if let Err(e) = hub.conversation.clear_messages() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("reset_messages failed: {e}"),
        );
    }
    (StatusCode::OK, "reset messages".to_string())
}

pub async fn post_reset_stories(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(orchestrator): axum::Extension<sync::OrchestratorHandle>,
    axum::Extension(select): axum::Extension<store::Select>,
) -> impl IntoResponse {
    let Some(refresh) = orchestrator.as_ref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "reset_stories: tracker not configured (test-refresh handle absent)".to_string(),
        );
    };
    refresh.clear_stories();
    let invalidate_delta = message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::BoardInvalidated {
            reason: "board reset".to_string(),
        }),
        ..message::Message::default()
    };
    if let Err(e) = hub.broadcast(&invalidate_delta) {
        diag::log_diag(&format!(
            "post_reset_stories: board_invalidated state-delta broadcast failed: {e}"
        ));
    }
    crate::rest::shared::broadcast_stories(&hub, &select);
    (StatusCode::OK, "reset stories".to_string())
}

#[derive(Deserialize)]
pub struct SeedRosterBody {
    pub identities: Vec<String>,
    #[serde(default)]
    pub presence: Option<store::Presence>,
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
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(orchestrator): axum::Extension<sync::OrchestratorHandle>,
    axum::Extension(dispatcher): axum::Extension<store::Dispatch>,
    axum::Extension(select): axum::Extension<store::Select>,
    Json(body): Json<SeedStoriesBody>,
) -> impl IntoResponse {
    let Some(refresh) = orchestrator.as_ref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "seed_stories: tracker not configured (test-refresh handle absent)".to_string(),
        );
    };
    for s in &body.stories {
        if let Err(message) = seed_one_story(refresh, &dispatcher, s) {
            return (StatusCode::INTERNAL_SERVER_ERROR, message);
        }
    }
    let invalidate_delta = message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::BoardInvalidated {
            reason: "board changed".to_string(),
        }),
        ..message::Message::default()
    };
    if let Err(e) = hub.broadcast(&invalidate_delta) {
        diag::log_diag(&format!(
            "post_seed_stories: board_invalidated state-delta broadcast failed: {e}"
        ));
    }
    crate::rest::shared::broadcast_stories(&hub, &select);
    (
        StatusCode::CREATED,
        format!("seeded {} stories", body.stories.len()),
    )
}

pub async fn post_seed_roster(
    axum::Extension(store): axum::Extension<Arc<store::Store>>,
    Json(body): Json<SeedRosterBody>,
) -> impl IntoResponse {
    for identity in &body.identities {
        store
            .dispatch(store::AppAction::Roster(
                store::RosterAction::RosterMemberUpserted {
                    routing_id: identity.clone(),
                    identity: identity.clone(),
                    presence: body.presence.unwrap_or(store::Presence::Online),
                },
            ))
            .await;
    }
    (
        StatusCode::OK,
        format!("seeded {} roster identities", body.identities.len()),
    )
}

#[derive(serde::Deserialize)]
pub struct SeedTransition {
    pub story_id: u64,
    pub new_state: String,
    pub observed_at: String,
}

pub async fn post_seed_transitions(
    axum::extract::Extension(transitions): axum::extract::Extension<
        std::sync::Arc<product_management::data_gateway::TransitionsDataGateway>,
    >,
    axum::Json(rows): axum::Json<Vec<SeedTransition>>,
) -> impl axum::response::IntoResponse {
    for row in &rows {
        let Ok(at) = chrono::DateTime::parse_from_rfc3339(&row.observed_at) else {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                format!("bad observed_at: {}", row.observed_at),
            );
        };
        transitions.record(row.story_id, &row.new_state, at.timestamp());
    }
    (axum::http::StatusCode::OK, format!("seeded {}", rows.len()))
}

fn seed_meta() -> product_management::story::events::EventMeta {
    product_management::story::events::EventMeta {
        actor: "test-seed".to_string(),
        actor_id: None,
        ts: chrono::Utc::now(),
    }
}

fn seed_initial_state(raw: &str) -> product_management::story::Work {
    use product_management::story::Work;
    match raw {
        "in-progress" | "desk-check" => Work::InProgress,
        "delivered" | "acceptance" => Work::Acceptance,
        "accepted" => Work::Accepted,
        _ => Work::Ready,
    }
}

fn seed_story_kind(raw: &str) -> product_management::story::StoryKind {
    use product_management::story::StoryKind;
    match raw {
        "bug" => StoryKind::Bug,
        "chore" => StoryKind::Chore,
        _ => StoryKind::Feature,
    }
}

fn seed_one_story(
    refresh: &sync::SyncOrchestrator,
    dispatcher: &store::Dispatch,
    s: &SeedStory,
) -> Result<(), String> {
    let initial_state = seed_initial_state(&s.initial_state);
    let story_type = seed_story_kind(&s.story_type);
    let is_iteration_seed = s.story_type.as_str() == "iteration";
    let is_epic_seed = s.story_type.as_str() == "epic";
    refresh
        .mutate(product_management::story::events::StoryEvent::new(
            seed_meta(),
            product_management::story::StoryId::new(s.id),
            product_management::story::events::StoryEventKind::StoryCreated {
                title: s.title.clone(),
                body: s.body.clone(),
                initial_state: initial_state.clone(),
                story_type,
            },
        ))
        .map_err(|e| format!("seed_stories: StoryCreated #{} failed: {e}", s.id))?;
    if s.initial_state == "desk-check" {
        dispatcher.dispatch(store::AppAction::Board(board::BoardAction::StoryPlaced {
            story: product_management::story::StoryId::new(s.id),
            stage: product_management::story::Stage::DeskCheck.id(),
        }));
    }
    let type_label = if is_iteration_seed {
        "iteration".to_string()
    } else if is_epic_seed {
        "epic".to_string()
    } else {
        source_codec::gh_label_codec::GhLabelCodec.encode_story_type(story_type)
    };
    refresh
        .mutate(product_management::story::events::StoryEvent::new(
            seed_meta(),
            product_management::story::StoryId::new(s.id),
            product_management::story::events::StoryEventKind::TagAdded { label: type_label },
        ))
        .map_err(|e| format!("seed_stories: type-label TagAdded on #{} failed: {e}", s.id))?;
    seed_labels(refresh, s)?;
    seed_closed(refresh, s)?;
    if is_iteration_seed || is_epic_seed {
        seed_container(dispatcher, s, initial_state, is_iteration_seed);
    }
    Ok(())
}

fn seed_labels(refresh: &sync::SyncOrchestrator, s: &SeedStory) -> Result<(), String> {
    let codec = source_codec::gh_label_codec::GhLabelCodec;
    for label in &s.labels {
        if codec.is_iteration_label_loose(label) {
            continue;
        }
        refresh
            .mutate(product_management::story::events::StoryEvent::new(
                seed_meta(),
                product_management::story::StoryId::new(s.id),
                product_management::story::events::StoryEventKind::TagAdded {
                    label: label.clone(),
                },
            ))
            .map_err(|e| format!("seed_stories: label event {label} on #{} failed: {e}", s.id))?;
    }
    Ok(())
}

fn seed_closed(refresh: &sync::SyncOrchestrator, s: &SeedStory) -> Result<(), String> {
    if !s.is_closed {
        return Ok(());
    }
    refresh
        .mutate(product_management::story::events::StoryEvent::new(
            seed_meta(),
            product_management::story::StoryId::new(s.id),
            product_management::story::events::StoryEventKind::StoryClosed {
                reason: Some("test-seed: closed at seed time".to_string()),
            },
        ))
        .map_err(|e| format!("seed_stories: StoryClosed #{} failed: {e}", s.id))
        .map(|_| ())
}

fn seed_container(
    dispatcher: &store::Dispatch,
    s: &SeedStory,
    initial_state: product_management::story::Work,
    is_iteration_seed: bool,
) {
    let mut carrier = product_management::story::Story::new_minimal(
        product_management::story::StoryId::new(s.id),
        s.title.clone(),
        initial_state,
        chrono::Utc::now(),
        "test-seed".to_string(),
    );
    carrier.body = Some(s.body.clone());
    if is_iteration_seed {
        dispatcher.dispatch(store::AppAction::Iteration(
            product_management::iteration::IterationAction::Upserted(vec![
                source_codec::mapper::decode_iteration_from_story(&carrier, &s.labels, s.is_closed),
            ]),
        ));
    } else {
        dispatcher.dispatch(store::AppAction::Epic(
            product_management::epic::EpicAction::Upserted(vec![
                source_codec::mapper::decode_epic_from_story(&carrier, s.is_closed),
            ]),
        ));
    }
}
