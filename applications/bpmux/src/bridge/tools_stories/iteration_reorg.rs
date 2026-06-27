use super::helpers::{ensure_story_in_canonical_store, require_purpose};
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::events::EventMeta;
use crate::product_management::board::{BoardEvent, PeerId};
use crate::product_management::story::{StoryState, StoryType};
use std::sync::Arc;

fn meta_for(agent_name: &str) -> EventMeta {
    EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    }
}

async fn member_of(
    r: &Arc<dyn RefreshDriver>,
    iteration_number: u64,
    story_number: u64,
) -> Result<crate::product_management::story::Story, String> {
    let iteration = ensure_story_in_canonical_store(r, iteration_number).await?;
    let story_type = iteration.story_type();
    if story_type != StoryType::Iteration {
        return Err(format!(
            "#{iteration_number} is not an Iteration (story_type={:?}); only iterations carry members",
            story_type
        ));
    }
    let story = ensure_story_in_canonical_store(r, story_number).await?;
    if crate::product_management::board::adapters::github::iteration_membership_from_labels(
        &story.raw_labels,
    )
    .is_none_or(|(it, _)| it != iteration_number)
    {
        return Err(format!(
            "#{story_number} is not a member of iteration #{iteration_number}"
        ));
    }
    Ok(story)
}

pub(crate) async fn handle_remove_iteration_member(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "remove an iteration member")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_number' argument")?;
    let story_number = args
        .get("story_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'story_number' argument")?;
    let reason = args
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .filter(|r| !r.trim().is_empty())
        .ok_or("remove-member needs --reason <R> (destructive scope-cut; the reason is the audit-trail)")?;

    let r = refresh.ok_or("remove_iteration_member requires refresh (RefreshDriver)")?;
    let story = member_of(r, iteration_number, story_number).await?;

    r.mutate_sync(BoardEvent::IterationMembershipCleared {
        meta: meta_for(agent_name),
        number: story_number,
    })
    .map_err(|e| format!("#{story_number} membership-clear enqueue failed: {e}"))?;

    let state_flag = match story.state {
        StoryState::Icebox | StoryState::Ready => String::new(),
        other => format!(" (was: {})", other.as_str()),
    };
    Ok(format!(
        "#{story_number} removed from iteration #{iteration_number} (reason: {reason}){state_flag}; the story is otherwise untouched"
    ))
}

pub(crate) async fn handle_move_iteration_member(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "reorder an iteration member")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_number' argument")?;
    let story_number = args
        .get("story_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'story_number' argument")?;
    let position =
        args.get("position")
            .and_then(serde_json::Value::as_u64)
            .ok_or("move-member needs --to <P> (the new 1-indexed position)")? as u32;

    let r = refresh.ok_or("move_iteration_member requires refresh (RefreshDriver)")?;
    member_of(r, iteration_number, story_number).await?;

    r.mutate_sync(BoardEvent::IterationMembershipSet {
        meta: meta_for(agent_name),
        number: story_number,
        iteration_number,
        position,
    })
    .map_err(|e| format!("#{story_number} reorder enqueue failed: {e}"))?;
    Ok(format!(
        "#{story_number} moved to position {position} in iteration #{iteration_number}"
    ))
}

pub(crate) async fn handle_rename_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "edit an iteration name")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_number' argument")?;
    let name = args
        .get("name")
        .and_then(serde_json::Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .ok_or("iteration rename needs --name <N>")?;

    let r = refresh.ok_or("rename_iteration requires refresh (RefreshDriver)")?;
    let iteration = ensure_story_in_canonical_store(r, iteration_number).await?;
    let story_type = iteration.story_type();
    if story_type != StoryType::Iteration {
        return Err(format!(
            "#{iteration_number} is not an Iteration (story_type={:?})",
            story_type
        ));
    }
    r.update_title(iteration_number, format!("Iteration {name}"))
        .await
        .map_err(|e| format!("#{iteration_number} name update failed: {e}"))?;
    Ok(format!("iteration #{iteration_number} renamed to {name}"))
}
