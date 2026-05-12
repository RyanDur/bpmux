use std::sync::Arc;

use crate::board::domain::StoryType;
use crate::board::{BoardStore, Story};

pub(in crate::bridge) async fn handle_list_iteration_children(
    args: &serde_json::Map<String, serde_json::Value>,
    store: &Arc<BoardStore>,
) -> Result<String, String> {
    let iteration_number = args
        .get("iteration_number")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            "list_iteration_children: iteration_number argument required (u64)".to_string()
        })?;
    let iteration = store.story(iteration_number).ok_or_else(|| {
        format!("list_iteration_children: iteration_number {iteration_number} not found")
    })?;
    if !matches!(iteration.story_type, StoryType::Iteration) {
        return Err(format!(
            "list_iteration_children: #{iteration_number} is not an Iteration (story_type={:?})",
            iteration.story_type
        ));
    }
    let mut children: Vec<Story> = store
        .all_stories()
        .into_iter()
        .filter(|s| s.iteration_id == Some(iteration_number))
        .collect();
    children.sort_by_key(|s| s.number);
    let body = serde_json::json!({
        "iteration_number": iteration_number,
        "children": children
            .iter()
            .map(|s| serde_json::json!({
                "number": s.number,
                "title": s.title,
                "state": format!("{:?}", s.state).to_lowercase(),
                "story_type": format!("{:?}", s.story_type).to_lowercase(),
                "is_closed": s.is_closed,
                "parent_epic": s.parent_epic,
            }))
            .collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&body).map_err(|e| format!("serialize: {e}"))
}
