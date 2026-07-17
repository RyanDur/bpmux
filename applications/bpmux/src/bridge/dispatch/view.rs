use std::sync::Arc;

use super::super::{tools, tools_stories};

pub(crate) fn dispatch_view_velocity(select: &crate::store::Select) -> Result<String, String> {
    let (shared, local, iterations, feature_ids) = select.select(|state| {
        (
            state.shared_history.clone(),
            state.local_history.clone(),
            product_management::iteration::all(&state.iterations),
            product_management::story::slice::feature_ids(&state.stories),
        )
    });
    let view =
        tools::build_velocity_view(&shared, &local, &iterations, |id| feature_ids.contains(&id));
    serde_json::to_string_pretty(&view).map_err(|e| format!("serialize view_velocity: {e}"))
}

pub(crate) async fn dispatch_view_story_commits(
    args: serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    tools::handle_view_story_commits(&args).await
}

pub(crate) async fn dispatch_view_commit_diff(
    args: serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    tools::handle_view_commit_diff(&args).await
}

pub(crate) async fn dispatch_view_board(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<crate::sync::SyncOrchestrator>>,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools_stories::handle_view_board(&args, refresh, select).await
}

pub(crate) async fn dispatch_get_story(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<crate::sync::SyncOrchestrator>>,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools::handle_get_story(&args, refresh, select).await
}

pub(crate) async fn dispatch_list_ready(select: &crate::store::Select) -> Result<String, String> {
    let iterations = select.select(|state| product_management::iteration::all(&state.iterations));
    tools::handle_list_ready(select, &iterations).await
}

pub(crate) async fn dispatch_list_epic_children(
    args: serde_json::Map<String, serde_json::Value>,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools::handle_list_epic_children(&args, select).await
}

pub(crate) async fn dispatch_dev_in_flight(
    select: &crate::store::Select,
) -> Result<String, String> {
    tools::handle_dev_in_flight(select).await
}

pub(crate) async fn dispatch_view_story_timeline(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<crate::sync::SyncOrchestrator>>,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools::handle_view_story_timeline(&args, refresh, select).await
}

pub(crate) async fn dispatch_view_comments(
    args: serde_json::Map<String, serde_json::Value>,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools::handle_view_comments(&args, select).await
}

pub(crate) async fn dispatch_view_backlog(select: &crate::store::Select) -> Result<String, String> {
    let (stories, iterations) = select.select(|state| {
        (
            product_management::story::slice::all(&state.stories),
            product_management::iteration::all(&state.iterations),
        )
    });
    let summary = tools::build_backlog_summary(&stories, &iterations);
    serde_json::to_string(&summary).map_err(|e| format!("serialize summary: {e}"))
}

pub(crate) async fn dispatch_view_verdicts(
    args: serde_json::Map<String, serde_json::Value>,
    select: &crate::store::Select,
) -> Result<String, String> {
    tools::handle_view_verdicts(&args, select).await
}
