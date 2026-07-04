use std::sync::Arc;

use super::super::tools_stories;

pub(crate) async fn dispatch_create_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_create_outcome(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_map_iteration_to_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_map_iteration_to_outcome(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_reprioritize_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_reprioritize_outcome(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_edit_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_edit_outcome(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_retire_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_retire_outcome(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_unmap_outcome_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_unmap_outcome_iteration(&args, agent_name, refresh, dispatch).await
}

pub(crate) async fn dispatch_reorder_outcome_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_reorder_outcome_iteration(&args, agent_name, refresh, dispatch).await
}
