use std::sync::Arc;

use crate::board::iteration_label::current_iteration_number;
use crate::board::{BoardStore, StoryState};

pub(in crate::bridge) async fn handle_list_ready(
    store: &Arc<BoardStore>,
) -> Result<String, String> {
    let all = store.all_stories();
    let current = current_iteration_number(&all);
    let mut ready_stories = store.stories_in_state(StoryState::Ready);
    ready_stories.sort_by_cached_key(|s| match (current, s.iteration_membership) {
        (Some(ci), Some((it, pos))) if it == ci => (0u8, pos, s.number),
        _ => (1u8, u32::MAX, s.number),
    });
    let queue: Vec<serde_json::Value> = ready_stories
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            serde_json::json!({
                "position": idx + 1,
                "number": s.number,
                "title": s.title,
            })
        })
        .collect();
    serde_json::to_string_pretty(&queue).map_err(|e| format!("serialize: {e}"))
}
