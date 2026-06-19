use std::sync::Arc;

use super::super::{move_story, tools, tools_stories};

pub(crate) async fn dispatch_test_create_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_create_story(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_test_comment_on_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_comment_on_story(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_test_close_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_close_story(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_test_label_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_label_story(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_test_reorder_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_reorder_story(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_test_move_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    move_story::handle_move_story(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_test_edit_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_edit_story(&args, agent_name, refresh).await
}

pub(crate) async fn dispatch_test_submit_verdict(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    tools::handle_submit_verdict(&args, agent_name, session).await
}
