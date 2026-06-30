use std::sync::Arc;

use crate::product_management::board::BoardStore;
use crate::product_management::epic::EpicId;
use crate::product_management::story::{Story, StoryId};

pub(in crate::bridge) async fn handle_list_epic_children(
    args: &serde_json::Map<String, serde_json::Value>,
    store: &Arc<BoardStore>,
) -> Result<String, String> {
    let epic_id = args
        .get("epic_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "list_epic_children: epic_id argument required (u64)".to_string())?;
    store
        .epic(StoryId::new(epic_id))
        .ok_or_else(|| format!("list_epic_children: epic_id {epic_id} not found or not an Epic"))?;
    let mut children: Vec<Story> = store
        .all_stories()
        .into_iter()
        .filter(|s| s.parent_epic() == Some(EpicId::new(epic_id)))
        .collect();
    children.sort_by_key(|s| s.id);
    let body = serde_json::json!({
        "epic_id": epic_id,
        "children": children
            .iter()
            .map(|s| serde_json::json!({
                "id": s.id,
                "title": s.title,
                "state": format!("{:?}", s.state).to_lowercase(),
                "story_type": s.story_type().as_label(),
                "is_closed": s.is_closed,
            }))
            .collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&body).map_err(|e| format!("serialize: {e}"))
}
