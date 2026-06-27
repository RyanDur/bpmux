use std::sync::Arc;

use crate::product_management::board::BoardStore;
use crate::product_management::story::{Story, StoryType};

pub(in crate::bridge) async fn handle_list_epic_children(
    args: &serde_json::Map<String, serde_json::Value>,
    store: &Arc<BoardStore>,
) -> Result<String, String> {
    let epic_number = args
        .get("epic_number")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "list_epic_children: epic_number argument required (u64)".to_string())?;
    let epic = store
        .story(epic_number)
        .ok_or_else(|| format!("list_epic_children: epic_number {epic_number} not found"))?;
    let story_type = epic.story_type();
    if !matches!(story_type, StoryType::Epic) {
        return Err(format!(
            "list_epic_children: #{epic_number} is not an Epic (story_type={:?})",
            story_type
        ));
    }
    let mut children: Vec<Story> = store
        .all_stories()
        .into_iter()
        .filter(|s| s.parent_epic() == Some(epic_number))
        .collect();
    children.sort_by_key(|s| s.number);
    let body = serde_json::json!({
        "epic_number": epic_number,
        "children": children
            .iter()
            .map(|s| serde_json::json!({
                "number": s.number,
                "title": s.title,
                "state": format!("{:?}", s.state).to_lowercase(),
                "story_type": s.story_type().as_label(),
                "is_closed": s.is_closed,
            }))
            .collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&body).map_err(|e| format!("serialize: {e}"))
}
