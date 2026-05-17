use std::sync::Arc;

use crate::board::adapters::gh_label_codec::GhLabelCodec;
use crate::board::label_codec::LabelCodec;
use crate::board::{BoardStore, StoryState};

const READY_STATE: &str = "ready";

pub fn build_ready_queue<C: LabelCodec>(
    codec: &C,
    stories: Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    let ready: Vec<serde_json::Value> = stories
        .into_iter()
        .filter(|s| s.get("state").and_then(|v| v.as_str()) == Some(READY_STATE))
        .collect();
    let ordered = sort_ready_by_label(codec, ready);
    assign_positions(ordered)
}

fn sort_ready_by_label<C: LabelCodec>(
    codec: &C,
    mut stories: Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    stories.sort_by_cached_key(|s| {
        let label_pos = ready_order_from_value(codec, s).map(u64::from);
        let number = s.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
        (label_pos.unwrap_or(u64::MAX), number)
    });
    stories
}

fn ready_order_from_value<C: LabelCodec>(codec: &C, s: &serde_json::Value) -> Option<u32> {
    s.get("labels")?.as_array()?.iter().find_map(|l| {
        let name = l.as_str().or_else(|| l.get("name")?.as_str())?;
        codec.decode_ready_order(name)
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
    let codec = GhLabelCodec;
    let ready_stories = store.stories_in_state(StoryState::Ready);
    let json_stories: Vec<serde_json::Value> = ready_stories
        .iter()
        .map(|s| {
            let mut labels: Vec<serde_json::Value> = vec![serde_json::json!({"name": "ready"})];
            for raw in &s.raw_labels {
                if codec.is_ready_order_label(raw) {
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
    let queue = build_ready_queue(&codec, json_stories);
    serde_json::to_string_pretty(&queue).map_err(|e| format!("serialize: {e}"))
}
