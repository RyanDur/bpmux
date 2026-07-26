use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LessonsQuery {
    pub topic: Option<String>,
    pub limit: Option<usize>,
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

pub async fn get_lessons(
    State(hub): State<Arc<hub::Hub>>,
    Extension(lessons_repo): Extension<Arc<dyn memory::lessons::LessonRepo>>,
    Query(params): Query<LessonsQuery>,
) -> impl IntoResponse {
    if let Some(limit) = params.limit {
        let mut lessons = lessons_repo.load_lessons().lessons;
        lessons.sort_by(|a, b| {
            b.reinforcement_count
                .cmp(&a.reinforcement_count)
                .then_with(|| b.established.cmp(&a.established))
        });
        lessons.truncate(limit);
        let mut text = format!("{} top lessons\n", lessons.len());
        let mut watch_topics = Vec::new();
        for l in &lessons {
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

    match hub.read_messages() {
        Ok(messages) => {
            let mut index = memory::lessons::build_index(&messages, &hub.owner_name);
            let saved = lessons_repo.load_lessons();
            index.lessons = saved.lessons;

            let groups = memory::lessons::group_by_topic(&index);
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

pub async fn get_peer_lessons(
    Extension(lessons_repo): Extension<Arc<dyn memory::lessons::LessonRepo>>,
    Query(params): Query<PeerLessonsQuery>,
) -> impl IntoResponse {
    let Some(peer_name) = params.peer else {
        let peers = lessons_repo.list_peers();
        return (
            StatusCode::OK,
            format!("available peers: {}", peers.join(", ")),
        );
    };

    let index = lessons_repo.load_peer_lessons(&peer_name);
    let groups = memory::lessons::group_by_topic(&index);
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

pub async fn post_lesson(
    Extension(lessons_repo): Extension<Arc<dyn memory::lessons::LessonRepo>>,
    Json(payload): Json<SaveLessonPayload>,
) -> impl IntoResponse {
    match lessons_repo.add_lesson(
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
