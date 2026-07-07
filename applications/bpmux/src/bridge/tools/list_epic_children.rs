use std::sync::Arc;

use crate::product_management::epic::EpicId;
use crate::product_management::story::Story;
use crate::sync::RefreshDriver;

pub(in crate::bridge) async fn handle_list_epic_children(
    args: &serde_json::Map<String, serde_json::Value>,
    refresh: &Arc<dyn RefreshDriver>,
) -> Result<String, String> {
    let epic_id = args
        .get("epic_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "list_epic_children: epic_id argument required (u64)".to_string())?;
    refresh
        .epic(EpicId::new(epic_id))
        .ok_or_else(|| format!("list_epic_children: epic_id {epic_id} not found or not an Epic"))?;
    let iterations = refresh.iterations();
    let mut children: Vec<Story> = refresh
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
                "state": format!(
                    "{:?}",
                    crate::product_management::scheduling::projected_state(s, &iterations)
                ).to_lowercase(),
                "story_type": s.story_type().as_label(),
                "is_closed": s.is_concluded(),
            }))
            .collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&body).map_err(|e| format!("serialize: {e}"))
}
