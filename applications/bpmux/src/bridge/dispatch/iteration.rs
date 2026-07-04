use std::sync::Arc;

use super::super::tools_stories;

pub(crate) async fn dispatch_create_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_create_iteration(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_close_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::helpers::require_purpose(agent_name, &["pm"], "close iteration")?;
    let r = refresh.ok_or("close_iteration requires refresh (RefreshDriver)")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(|v| v.as_u64())
        .ok_or("close_iteration requires iteration_id (canonical Iteration issue#).")?;
    let iteration = r
        .iteration(crate::product_management::iteration::IterationId::new(
            iteration_id,
        ))
        .ok_or_else(|| {
            if r.story(iteration_id).is_some() {
                format!("close_iteration: #{iteration_id} is not an Iteration")
            } else {
                format!("close_iteration: iteration_id #{iteration_id} not found")
            }
        })?;
    if iteration.complete {
        return Err(format!(
            "close_iteration: iteration #{iteration_id} is already closed"
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
    crate::transport::rest::iteration_close::close_iteration_canonical(r, dispatch, iteration_id)
        .await?;
    crate::transport::rest::shared::broadcast_board_invalidate(session);
    Ok(format!("iteration #{iteration_id} closed"))
}

pub(crate) async fn dispatch_set_parent_epic(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_set_parent_epic(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_unset_parent_epic(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_unset_parent_epic(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_add_story_to_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_add_story_to_iteration(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_designate_current_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_designate_current_iteration(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_close_pre_pickup_review(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_close_pre_pickup_review(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_dev_ok_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_dev_ok_iteration(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_remove_iteration_member(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_remove_iteration_member(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_move_iteration_member(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_move_iteration_member(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_rename_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_rename_iteration(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_reorder_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_reorder_iteration(&args, agent_name, refresh, dispatch).await
}
