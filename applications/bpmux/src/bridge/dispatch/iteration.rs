use std::sync::Arc;

use super::super::tools_stories;
use crate::transport::rest::iteration_close::CloseIterationError;

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
    hub: &Arc<crate::hub::Hub>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::helpers::require_purpose(agent_name, &["pm"], "close iteration")?;
    let r = refresh.ok_or("close_iteration requires refresh (RefreshDriver)")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(|v| v.as_u64())
        .ok_or("close_iteration requires iteration_id (canonical Iteration issue#).")?;
    crate::transport::rest::iteration_close::close_iteration(hub, r, dispatch, iteration_id)
        .await
        .map_err(|e| match e {
            CloseIterationError::NotAnIteration(id) => {
                format!("close_iteration: #{id} is not an Iteration")
            }
            CloseIterationError::NotFound(id) => {
                format!("close_iteration: iteration_id #{id} not found")
            }
            CloseIterationError::AlreadyClosed(id) => {
                format!("close_iteration: iteration #{id} is already closed")
            }
            CloseIterationError::ReadyQueueBlocked(_) => {
                "close_iteration blocked: ready queue must be empty before iteration close (#1258 guard)."
                    .to_string()
            }
            CloseIterationError::Canonical(msg) => msg,
        })?;
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
