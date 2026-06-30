use super::helpers::{ensure_story_in_canonical_store, require_purpose};
use crate::product_management::epic::EpicId;
use crate::product_management::story::StoryId;

pub(crate) async fn handle_set_parent_epic(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "set parent epic")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let epic_id = args
        .get("epic_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'epic_id' argument")?;
    if number == epic_id {
        return Err(format!("story #{number} cannot be its own parent epic"));
    }
    let r = refresh.ok_or("set_parent_epic requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(r, number).await?;
    let parent = ensure_story_in_canonical_store(r, epic_id).await?;
    let story_type = parent.story_type();
    if story_type != crate::product_management::story::StoryType::Epic {
        return Err(format!(
            "story #{epic_id} is not an Epic (story_type={:?}); only Epic stories can be parents",
            story_type
        ));
    }
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(
        crate::product_management::board::BoardEvent::ParentEpicSet {
            meta,
            id: StoryId::new(number),
            epic_id: EpicId::new(epic_id),
        },
    )
    .map_err(|e| format!("#{number} parent_epic enqueue failed: {e}"))?;
    Ok(format!("#{number} grouped under epic #{epic_id}"))
}

pub(crate) async fn handle_unset_parent_epic(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "unset parent epic")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let r = refresh.ok_or("unset_parent_epic requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(r, number).await?;
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(
        crate::product_management::board::BoardEvent::ParentEpicUnset {
            meta,
            id: StoryId::new(number),
        },
    )
    .map_err(|e| format!("#{number} parent_epic-unset enqueue failed: {e}"))?;
    Ok(format!("#{number} ungrouped from parent epic"))
}
