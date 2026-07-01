use std::sync::Arc;

use crate::product_management::board::BoardStore;
use crate::product_management::iteration::Iteration;
use crate::product_management::story::{Story, StoryState};

pub(in crate::bridge) async fn handle_list_ready(
    store: &Arc<BoardStore>,
    iterations: &[Iteration],
) -> Result<String, String> {
    let current_order = iterations
        .iter()
        .find(|iteration| iteration.is_current)
        .map(|iteration| iteration.stories.clone())
        .unwrap_or_default();
    let ready = store.stories_in_state(StoryState::Ready);
    let mut queue: Vec<&Story> = current_order
        .iter()
        .filter_map(|id| ready.iter().find(|s| s.id == *id))
        .collect();
    let mut loose: Vec<&Story> = ready
        .iter()
        .filter(|s| !current_order.contains(&s.id))
        .collect();
    loose.sort_by_key(|s| s.id);
    queue.extend(loose);
    let rows: Vec<serde_json::Value> = queue
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            serde_json::json!({
                "position": idx + 1,
                "id": s.id,
                "title": s.title,
            })
        })
        .collect();
    serde_json::to_string_pretty(&rows).map_err(|e| format!("serialize: {e}"))
}
