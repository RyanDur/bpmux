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
    let any_labeled = stories.iter().any(|s| ready_order_from_value(s).is_some());
    if any_labeled {
        stories.sort_by_cached_key(|s| {
            let label_pos = ready_order_from_value(s).map(u64::from);
            let number = s.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
            (label_pos.unwrap_or(u64::MAX), number)
        });
        return stories;
    }
    let ready_priority = priority.columns.get(READY_STATE);
    if let Some(pri) = ready_priority {
        stories.sort_by_cached_key(|s| {
            let number = s.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
            pri.iter().position(|&p| p == number).unwrap_or(usize::MAX)
        });
    }
    stories
}

fn ready_order_from_value(s: &serde_json::Value) -> Option<u32> {
    s.get("labels")?.as_array()?.iter().find_map(|l| {
        let name = l.as_str().or_else(|| l.get("name")?.as_str())?;
        let rest = name.strip_prefix("ready-order.")?;
        rest.parse::<u32>().ok()
    })
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
            let mut labels: Vec<serde_json::Value> = vec![serde_json::json!({"name": "ready"})];
            for raw in &s.raw_labels {
                if raw.starts_with("ready-order.") {
                    labels.push(serde_json::json!({"name": raw}));
                }
            }
            serde_json::json!({
                "number": s.number,
                "title": s.title,
                "state": "ready",
                "labels": labels,
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
