use std::sync::Arc;

use super::super::{tools, tools_stories};

fn require_refresh<'a>(
    refresh: Option<&'a Arc<dyn crate::sync::RefreshDriver>>,
    tool: &str,
) -> Result<&'a Arc<dyn crate::sync::RefreshDriver>, String> {
    refresh.ok_or_else(|| format!("{tool} requires canonical store"))
}

pub(crate) fn dispatch_view_velocity(select: &crate::store::Select) -> Result<String, String> {
    let (shared, local, iterations, feature_ids) = select.select(|state| {
        (
            state.shared_history.clone(),
            state.local_history.clone(),
            crate::product_management::iteration::all(&state.iterations),
            crate::product_management::story::slice::feature_ids(&state.stories),
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
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_view_board(&args, refresh).await
}

pub(crate) async fn dispatch_get_story(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    tools::handle_get_story(&args, refresh).await
}

pub(crate) async fn dispatch_list_ready(
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    let r = require_refresh(refresh, "list_ready")?;
    let iterations = r.iterations();
    tools::handle_list_ready(r, &iterations).await
}

pub(crate) async fn dispatch_list_epic_children(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    let r = refresh.ok_or("list_epic_children requires canonical store")?;
    tools::handle_list_epic_children(&args, r).await
}

pub(crate) async fn dispatch_dev_in_flight(
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    let r = require_refresh(refresh, "dev_in_flight")?;
    tools::handle_dev_in_flight(r).await
}

pub(crate) async fn dispatch_view_story_timeline(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    tools::handle_view_story_timeline(&args, refresh).await
}

pub(crate) async fn dispatch_view_comments(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    tools::handle_view_comments(&args, refresh).await
}

pub(crate) async fn dispatch_view_backlog(
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    let r = require_refresh(refresh, "view_backlog")?;
    let stories = r.all_stories();
    let iterations = r.iterations();
    let summary = tools::build_backlog_summary(&stories, &iterations);
    serde_json::to_string(&summary).map_err(|e| format!("serialize summary: {e}"))
}

pub(crate) async fn dispatch_view_verdicts(
    args: serde_json::Map<String, serde_json::Value>,
    session: &Arc<crate::session::Session>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    tools::handle_view_verdicts(&args, session, refresh).await
}
