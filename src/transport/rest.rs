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
}

#[derive(Deserialize)]
pub struct GetMessagesParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    /// Filter to chat messages only (excludes activity/system). Default: true.
    pub chat_only: Option<bool>,
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
            let mut filtered: Vec<_> = if chat_only {
                messages.into_iter().filter(|m| m.is_chat()).collect()
            } else {
                messages
            };
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
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    match session.read_messages() {
        Ok(messages) => {
            let message_class = params.message_type.as_deref().and_then(|t| match t {
                "correction" | "experience" => Some(session::MessageClass::Experience(
                    session::classifier::ExperienceKind::Correction,
                )),
                "decision" => Some(session::MessageClass::Decision),
                "discussion" => Some(session::MessageClass::Discussion),
                _ => None,
            });
            let filters = session::SearchFilters {
                query: params.query,
                sender: params.sender,
                after: params.after.and_then(|s| s.parse().ok()),
                before: params.before.and_then(|s| s.parse().ok()),
                channel: params.channel,
                message_class,
                insight_only: params.message_type.as_deref() == Some("insight"),
                owner_name: session.owner_name.clone(),
            };
            let results = session::search(&messages, &filters);
            (
                StatusCode::OK,
                serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to search: {e}"),
        ),
    }
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

#[derive(Deserialize)]
pub struct ClickPayload {
    pub x: u16,
    pub y: u16,
}

pub async fn simulate_click(
    axum::Extension(tx): axum::Extension<tokio::sync::mpsc::Sender<crossterm::event::Event>>,
    Json(payload): Json<ClickPayload>,
) -> impl IntoResponse {
    let event = crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: payload.x,
        row: payload.y,
        modifiers: crossterm::event::KeyModifiers::empty(),
    });
    match tx.send(event).await {
        Ok(()) => (
            StatusCode::OK,
            format!("click at ({}, {})", payload.x, payload.y),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "event loop not available".to_string(),
        ),
    }
}

#[derive(Deserialize)]
pub struct KeyPayload {
    pub key: String,
}

pub async fn simulate_key(
    axum::Extension(tx): axum::Extension<tokio::sync::mpsc::Sender<crossterm::event::Event>>,
    Json(payload): Json<KeyPayload>,
) -> impl IntoResponse {
    let key_code = match payload.key.as_str() {
        "enter" => crossterm::event::KeyCode::Enter,
        "escape" | "esc" => crossterm::event::KeyCode::Esc,
        "tab" => crossterm::event::KeyCode::Tab,
        "backspace" => crossterm::event::KeyCode::Backspace,
        "up" => crossterm::event::KeyCode::Up,
        "down" => crossterm::event::KeyCode::Down,
        "left" => crossterm::event::KeyCode::Left,
        "right" => crossterm::event::KeyCode::Right,
        s if s.len() == 1 => crossterm::event::KeyCode::Char(s.chars().next().unwrap_or(' ')),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                format!("unknown key: {}", payload.key),
            );
        }
    };
    let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
        key_code,
        crossterm::event::KeyModifiers::empty(),
    ));
    match tx.send(event).await {
        Ok(()) => (StatusCode::OK, format!("key: {}", payload.key)),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "event loop not available".to_string(),
        ),
    }
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
    let msg = session::Message {
        from: payload.from,
        text: payload.text,
        ts: chrono::Utc::now(),
        message_type: session::MessageType::from(payload.message_type.as_deref()),
        to: None,
        addressee: payload.addressee,
        explicit_privacy: false,
    };
    if let Err(e) = session.broadcast(&msg) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to broadcast: {e}"),
        );
    }
    (StatusCode::CREATED, "message sent".to_string())
}
