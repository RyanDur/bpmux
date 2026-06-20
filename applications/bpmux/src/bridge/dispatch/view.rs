use std::sync::Arc;

use super::super::{WsSink, tools, tools_stories};

fn require_store(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    tool: &str,
) -> Result<Arc<crate::board::BoardStore>, String> {
    refresh
        .map(|r| r.store())
        .ok_or_else(|| format!("{tool} requires canonical store"))
}

pub(crate) async fn dispatch_view_velocity(
    messages: Vec<crate::session::Message>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let no_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools::handle_view_velocity(|| async move { Ok(messages) }, &no_ws, refresh).await
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
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_view_board(&args, refresh).await
}

pub(crate) async fn dispatch_get_story(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh.map(|r| r.store());
    tools::handle_get_story(&args, store.as_ref(), refresh).await
}

pub(crate) async fn dispatch_list_ready(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = require_store(refresh, "list_ready")?;
    tools::handle_list_ready(&store).await
}

pub(crate) async fn dispatch_list_epic_children(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = require_store(refresh, "list_epic_children")?;
    tools::handle_list_epic_children(&args, &store).await
}

pub(crate) async fn dispatch_dev_in_flight(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = require_store(refresh, "dev_in_flight")?;
    tools::handle_dev_in_flight(&store).await
}

pub(crate) async fn dispatch_view_story_timeline(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh.map(|r| r.store());
    tools::handle_view_story_timeline(&args, store.as_ref(), refresh).await
}

pub(crate) async fn dispatch_view_comments(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh.map(|r| r.store());
    tools::handle_view_comments(&args, store.as_ref(), refresh).await
}

pub(crate) async fn dispatch_view_backlog(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = require_store(refresh, "view_backlog")?;
    let stories = store.all_stories();
    let summary = tools::build_backlog_summary(&stories);
    serde_json::to_string(&summary).map_err(|e| format!("serialize summary: {e}"))
}

pub(crate) async fn dispatch_view_verdicts(
    args: serde_json::Map<String, serde_json::Value>,
    session: &Arc<crate::session::Session>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools::handle_view_verdicts(&args, session, refresh).await
}
