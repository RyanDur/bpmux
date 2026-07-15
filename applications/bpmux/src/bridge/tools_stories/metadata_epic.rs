use super::event_meta::meta_for;
use super::helpers::{
    ensure_epic_in_canonical_store, ensure_story_in_canonical_store, require_purpose,
};
use product_management::epic::EpicId;
use product_management::story::StoryId;

pub(crate) async fn handle_set_parent_epic(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
) -> Result<String, String> {
    require_purpose(role, &["pm"], "set parent epic")?;
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let epic_id = args
        .get("epic_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'epic_id' argument")?;
    if id == epic_id {
        return Err(format!("story #{id} cannot be its own parent epic"));
    }
    let r = refresh.ok_or("set_parent_epic requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(r, id).await?;
    ensure_epic_in_canonical_store(r, select, epic_id).await?;
    let meta = meta_for(agent_name, actor_id);
    r.mutate_sync(product_management::story::events::StoryEvent::new(
        meta,
        StoryId::new(id),
        product_management::story::events::StoryEventKind::ParentEpicSet {
            epic_id: EpicId::new(epic_id),
        },
    ))
    .map_err(|e| format!("#{id} parent_epic enqueue failed: {e}"))?;
    Ok(format!("#{id} grouped under epic #{epic_id}"))
}

pub(crate) async fn handle_unset_parent_epic(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(role, &["pm"], "unset parent epic")?;
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let r = refresh.ok_or("unset_parent_epic requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(r, id).await?;
    let meta = meta_for(agent_name, actor_id);
    r.mutate_sync(product_management::story::events::StoryEvent::new(
        meta,
        StoryId::new(id),
        product_management::story::events::StoryEventKind::ParentEpicUnset,
    ))
    .map_err(|e| format!("#{id} parent_epic-unset enqueue failed: {e}"))?;
    Ok(format!("#{id} ungrouped from parent epic"))
}
