use std::sync::Arc;

use crate::board::{BoardStore, StoryState};
use crate::session::priority::{PriorityIndex, load_priority};

const READY_STATE: &str = "ready";

pub fn build_ready_queue(
    stories: Vec<serde_json::Value>,
    priority: &PriorityIndex,
) -> Vec<serde_json::Value> {
    let ready: Vec<serde_json::Value> = stories
        .into_iter()
        .filter(|s| s.get("state").and_then(|v| v.as_str()) == Some(READY_STATE))
        .collect();
    let ordered = sort_ready_by_priority(ready, priority);
    assign_positions(ordered)
}

fn sort_ready_by_priority(
    mut stories: Vec<serde_json::Value>,
    priority: &PriorityIndex,
) -> Vec<serde_json::Value> {
    let ready_priority = priority.columns.get(READY_STATE);
    if let Some(pri) = ready_priority {
        stories.sort_by_cached_key(|s| {
            let number = s.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
            pri.iter().position(|&p| p == number).unwrap_or(usize::MAX)
        });
    }
    stories
}

fn assign_positions(stories: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    stories
        .into_iter()
        .enumerate()
        .map(|(idx, s)| {
            serde_json::json!({
                "position": idx + 1,
                "number": s.get("number"),
                "title": s.get("title"),
                "labels": s.get("labels"),
            })
        })
        .collect()
}

pub(in crate::bridge) async fn handle_list_ready(
    store: &Arc<BoardStore>,
) -> Result<String, String> {
    let ready_stories = store.stories_in_state(StoryState::Ready);
    let json_stories: Vec<serde_json::Value> = ready_stories
        .iter()
        .map(|s| {
            serde_json::json!({
                "number": s.number,
                "title": s.title,
                "state": "ready",
                "labels": [{"name": "ready"}],
            })
        })
        .collect();
    let priority = match crate::session::priority::default_priority_path() {
        Ok(path) => load_priority(&path),
        Err(_) => PriorityIndex::default(),
    };
    let queue = build_ready_queue(json_stories, &priority);
    serde_json::to_string_pretty(&queue).map_err(|e| format!("serialize: {e}"))
}
