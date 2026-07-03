use super::helpers::{
    ensure_epic_in_canonical_store, ensure_story_in_canonical_store, require_purpose,
};
use crate::product_management::epic::EpicId;
use crate::product_management::story::StoryId;

pub(crate) async fn handle_set_parent_epic(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "set parent epic")?;
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
    ensure_epic_in_canonical_store(r, epic_id).await?;
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::routing::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(
        crate::product_management::board::StoryEvent::ParentEpicSet {
            meta,
            id: StoryId::new(id),
            epic_id: EpicId::new(epic_id),
        },
    )
    .map_err(|e| format!("#{id} parent_epic enqueue failed: {e}"))?;
    Ok(format!("#{id} grouped under epic #{epic_id}"))
}

pub(crate) async fn handle_unset_parent_epic(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "unset parent epic")?;
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let r = refresh.ok_or("unset_parent_epic requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(r, id).await?;
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::routing::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(
        crate::product_management::board::StoryEvent::ParentEpicUnset {
            meta,
            id: StoryId::new(id),
        },
    )
    .map_err(|e| format!("#{id} parent_epic-unset enqueue failed: {e}"))?;
    Ok(format!("#{id} ungrouped from parent epic"))
}
