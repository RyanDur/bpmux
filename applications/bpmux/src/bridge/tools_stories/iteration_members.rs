use super::helpers::{
    ensure_iteration_in_canonical_store, ensure_story_in_canonical_store, require_purpose,
};
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, Relationships,
};
use crate::product_management::iteration::Iteration;
use crate::product_management::story::{Story, StoryState};
use std::sync::Arc;

async fn member_of(
    r: &Arc<dyn RefreshDriver>,
    iteration_number: u64,
    story_number: u64,
) -> Result<(Iteration, Story), String> {
    let iteration = ensure_iteration_in_canonical_store(r, iteration_number).await?;
    let story = ensure_story_in_canonical_store(r, story_number).await?;
    if !iteration.stories.contains(&story_number) {
        return Err(format!(
            "#{story_number} is not a member of iteration #{iteration_number}"
        ));
    }
    Ok((iteration, story))
}

async fn rewrite_members(
    r: &Arc<dyn RefreshDriver>,
    iteration: &Iteration,
    stories: Vec<u64>,
) -> Result<(), String> {
    let number = iteration.id().saved_number().unwrap_or_default();
    let mut rel = iteration
        .content
        .body
        .as_deref()
        .and_then(IterationRelationships::read_from_body)
        .unwrap_or_default();
    rel.stories = stories;
    let body = rel.write_into_body(iteration.content.body.as_deref().unwrap_or_default());
    r.update_body(number, body)
        .await
        .map_err(|e| format!("#{number} iteration-member body update failed: {e}"))
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
    let (iteration, story) = member_of(r, iteration_number, story_number).await?;

    let mut stories = iteration.stories.clone();
    stories.retain(|n| *n != story_number);
    rewrite_members(r, &iteration, stories).await?;

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
    let (iteration, _) = member_of(r, iteration_number, story_number).await?;

    let mut stories = iteration.stories.clone();
    stories.retain(|n| *n != story_number);
    let insert_index = (position.max(1) as usize - 1).min(stories.len());
    stories.insert(insert_index, story_number);
    rewrite_members(r, &iteration, stories).await?;

    Ok(format!(
        "#{story_number} moved to position {position} in iteration #{iteration_number}"
    ))
}
