use super::helpers::{
    ensure_iteration_in_canonical_store, ensure_story_in_canonical_store, require_purpose,
};
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, Relationships,
};
use crate::product_management::iteration::Iteration;
use crate::product_management::story::{Story, StoryId, StoryState};
use std::sync::Arc;

async fn member_of(
    r: &Arc<dyn RefreshDriver>,
    iteration_id: u64,
    story_id: u64,
) -> Result<(Iteration, Story), String> {
    let iteration = ensure_iteration_in_canonical_store(r, iteration_id).await?;
    let story = ensure_story_in_canonical_store(r, story_id).await?;
    if !iteration.stories.contains(&StoryId::new(story_id)) {
        return Err(format!(
            "#{story_id} is not a member of iteration #{iteration_id}"
        ));
    }
    Ok((iteration, story))
}

async fn rewrite_members(
    r: &Arc<dyn RefreshDriver>,
    iteration: &Iteration,
    stories: Vec<StoryId>,
) -> Result<(), String> {
    let id = iteration
        .id()
        .saved_id()
        .map(|id| id.value())
        .unwrap_or_default();
    let mut rel = iteration
        .content
        .body
        .as_deref()
        .and_then(IterationRelationships::read_from_body)
        .unwrap_or_default();
    rel.stories = stories;
    let body = rel.write_into_body(iteration.content.body.as_deref().unwrap_or_default());
    r.update_body(id, body)
        .await
        .map_err(|e| format!("#{id} iteration-member body update failed: {e}"))
}

pub(crate) async fn set_member_position(
    r: &Arc<dyn RefreshDriver>,
    iteration_id: u64,
    story_id: u64,
    position: u32,
) -> Result<(), String> {
    let target = StoryId::new(story_id);
    let iteration = ensure_iteration_in_canonical_store(r, iteration_id).await?;
    for other in r.iterations() {
        let Some(other_id) = other.id().saved_id().map(|id| id.value()) else {
            continue;
        };
        if other_id == iteration_id || !other.stories.contains(&target) {
            continue;
        }
        let mut other_stories = other.stories.clone();
        other_stories.retain(|n| *n != target);
        rewrite_members(r, &other, other_stories).await?;
    }
    let mut stories = iteration.stories.clone();
    stories.retain(|n| *n != target);
    let insert_index = (position.max(1) as usize - 1).min(stories.len());
    stories.insert(insert_index, target);
    rewrite_members(r, &iteration, stories).await
}

pub(crate) async fn handle_remove_iteration_member(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "remove an iteration member")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_id' argument")?;
    let story_id = args
        .get("story_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'story_id' argument")?;
    let reason = args
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .filter(|r| !r.trim().is_empty())
        .ok_or("remove-member needs --reason <R> (destructive scope-cut; the reason is the audit-trail)")?;

    let r = refresh.ok_or("remove_iteration_member requires refresh (RefreshDriver)")?;
    let (iteration, story) = member_of(r, iteration_id, story_id).await?;

    let mut stories = iteration.stories.clone();
    stories.retain(|n| *n != StoryId::new(story_id));
    rewrite_members(r, &iteration, stories).await?;

    let state_flag = match story.state {
        StoryState::Icebox | StoryState::Ready => String::new(),
        other => format!(" (was: {})", other.as_str()),
    };
    Ok(format!(
        "#{story_id} removed from iteration #{iteration_id} (reason: {reason}){state_flag}; the story is otherwise untouched"
    ))
}

pub(crate) async fn handle_move_iteration_member(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "reorder an iteration member")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_id' argument")?;
    let story_id = args
        .get("story_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'story_id' argument")?;
    let position =
        args.get("position")
            .and_then(serde_json::Value::as_u64)
            .ok_or("move-member needs --to <P> (the new 1-indexed position)")? as u32;

    let r = refresh.ok_or("move_iteration_member requires refresh (RefreshDriver)")?;
    let (iteration, _) = member_of(r, iteration_id, story_id).await?;

    let mut stories = iteration.stories.clone();
    stories.retain(|n| *n != StoryId::new(story_id));
    let insert_index = (position.max(1) as usize - 1).min(stories.len());
    stories.insert(insert_index, StoryId::new(story_id));
    rewrite_members(r, &iteration, stories).await?;

    Ok(format!(
        "#{story_id} moved to position {position} in iteration #{iteration_id}"
    ))
}
