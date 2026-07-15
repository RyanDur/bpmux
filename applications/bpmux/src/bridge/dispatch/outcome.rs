use std::sync::Arc;

use super::super::tools_stories;

pub(crate) async fn dispatch_create_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    tools_stories::handle_create_outcome(&args, agent_name, role, actor_id, refresh, dispatch).await
}

pub(crate) async fn dispatch_map_iteration_to_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    role: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools_stories::handle_map_iteration_to_outcome(&args, role, refresh, dispatch, select).await
}

pub(crate) async fn dispatch_reprioritize_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools_stories::handle_reprioritize_outcome(
        &args, agent_name, role, actor_id, refresh, dispatch, select,
    )
    .await
}

pub(crate) async fn dispatch_edit_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    role: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools_stories::handle_edit_outcome(&args, role, refresh, dispatch, select).await
}

pub(crate) async fn dispatch_retire_outcome(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools_stories::handle_retire_outcome(
        &args, agent_name, role, actor_id, refresh, dispatch, select,
    )
    .await
}

pub(crate) async fn dispatch_unmap_outcome_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    role: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools_stories::handle_unmap_outcome_iteration(&args, role, refresh, dispatch, select).await
}

pub(crate) async fn dispatch_reorder_outcome_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    role: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools_stories::handle_reorder_outcome_iteration(&args, role, refresh, dispatch, select).await
}
