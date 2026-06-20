use std::sync::Arc;

use super::super::tools_stories;

pub(crate) async fn dispatch_create_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_create_iteration(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_create_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_create_outcome(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_close_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    tools_stories::helpers::require_purpose(agent_name, &["pm"], "close iteration")?;
    let r = refresh.ok_or("close_iteration requires refresh (RefreshDriver)")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(|v| v.as_u64())
        .ok_or("close_iteration requires iteration_number (canonical Iteration issue#).")?;
    let iteration = r.story(iteration_number).ok_or_else(|| {
        format!("close_iteration: iteration_number #{iteration_number} not found")
    })?;
    if !matches!(
        iteration.story_type,
        crate::board::domain::StoryType::Iteration
    ) {
        return Err(format!(
            "close_iteration: #{iteration_number} is not an Iteration (story_type={:?})",
            iteration.story_type
        ));
    }
    if iteration.is_closed {
        return Err(format!(
            "close_iteration: iteration #{iteration_number} is already closed"
        ));
    }
    let blockers =
        crate::transport::rest::iteration_close::collect_ready_queue_blockers(r.as_ref()).await;
    if !blockers.is_empty() {
        return Err(
            "close_iteration blocked: ready queue must be empty before iteration close (#1258 guard)."
                .to_string(),
        );
    }
    crate::transport::rest::iteration_close::close_iteration_canonical(r, iteration_number).await?;
    crate::transport::rest::shared::broadcast_board_invalidate(session);
    Ok(format!("iteration #{iteration_number} closed"))
}

pub(crate) async fn dispatch_set_parent_epic(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_set_parent_epic(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_unset_parent_epic(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_unset_parent_epic(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_add_story_to_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_add_story_to_iteration(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_designate_current_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_designate_current_iteration(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_close_pre_pickup_review(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_close_pre_pickup_review(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_dev_ok_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_dev_ok_iteration(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_map_iteration_to_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_map_iteration_to_outcome(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_reprioritize_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_reprioritize_outcome(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_edit_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_edit_outcome(&args, agent_name, refresh).await
}
