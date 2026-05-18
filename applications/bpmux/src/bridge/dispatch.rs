//! Test-RPC dispatch glue: thin async wrappers around the
//! `tools_stories::handle_*` and `tools::handle_*` handlers used by
//! `/api/test/*` REST endpoints and the MCP RPC route.
//!
//! Each `dispatch_test_*` function builds an empty `WsSink` mutex
//! (REST dispatch has no client WS to fire signals to) and forwards
//! to the underlying handler. Post-handler, workflow signals that the
//! handler would have fired via `ws_send` are instead broadcast
//! through `session.broadcast` so chat observers see the same
//! transitions REST observers see.

use std::sync::Arc;

use super::{WsSink, move_story, tools, tools_stories};

/// #570 helper: broadcast a `ClientMessage::Send` workflow signal via
/// `session.broadcast` (canonical-store-side path). Used by dispatch_
/// test_* after handlers — handler-internal `ws_send(empty_ws_tx, ...)`
/// is silent post-#516 because RPC dispatch passes None ws_tx; this
/// helper restores the chat-side workflow-signal observation surface.
pub(super) fn broadcast_client_msg_via_session(
    session: &Arc<crate::session::Session>,
    agent_name: &str,
    msg: crate::transport::protocol::ClientMessage,
) {
    if let crate::transport::protocol::ClientMessage::Send {
        text,
        to,
        addressee,
        message_type,
    } = msg
    {
        let explicit_privacy = to.is_some();
        let from_short = crate::transport::ws::short_name(agent_name);
        let session_msg = crate::session::Message {
            from: format!("{from_short}:claude-code"),
            text,
            ts: chrono::Utc::now(),
            message_type: crate::session::message::MessageType::from(message_type.as_deref()),
            to,
            addressee,
            explicit_privacy,
            ..crate::session::Message::default()
        };
        if let Err(e) = session.broadcast(&session_msg) {
            crate::session::diag::log_diag(&format!("rpc: workflow-signal broadcast failed: {e}"));
        }
    }
}

pub(crate) async fn dispatch_test_create_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_create_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_create_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_create_iteration(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_comment_on_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_comment_on_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_close_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_close_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_deliver_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result = tools_stories::handle_deliver_story(&args, agent_name, &empty_ws, refresh).await?;
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        let title = refresh
            .and_then(|r| r.story(number))
            .map(|s| s.title)
            .unwrap_or_else(|| format!("#{number}"));
        for msg in tools_stories::build_delivery_signals(number, &title) {
            broadcast_client_msg_via_session(session, agent_name, msg);
        }
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_start_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result = tools_stories::handle_start_story(&args, agent_name, &empty_ws, refresh).await?;
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        broadcast_client_msg_via_session(
            session,
            agent_name,
            tools_stories::build_start_signal(number),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_accept_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result =
        tools_stories::handle_accept_story(&args, agent_name, &empty_ws, refresh, Some(session))
            .await?;
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        let role = tools_stories::role_from_name(agent_name);
        broadcast_client_msg_via_session(
            session,
            agent_name,
            tools_stories::build_accept_signal(number, role),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_fail_desk_check(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result =
        tools_stories::handle_fail_desk_check(&args, agent_name, &empty_ws, refresh).await?;
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        let reason = args
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("needs fixes");
        broadcast_client_msg_via_session(
            session,
            agent_name,
            tools_stories::build_fail_desk_check_signal(number, reason),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_pass_desk_check(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result =
        tools_stories::handle_pass_desk_check(&args, agent_name, &empty_ws, refresh, Some(session))
            .await?;
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        let story_type = tools_stories::classify_for_pass_desk_check(number, refresh);
        for msg in tools_stories::build_pass_desk_check_signals(number, story_type) {
            broadcast_client_msg_via_session(session, agent_name, msg);
        }
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_reject_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result = tools_stories::handle_reject_story(&args, agent_name, &empty_ws, refresh).await?;
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        let reason = args
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("rejected");
        broadcast_client_msg_via_session(
            session,
            agent_name,
            tools_stories::build_reject_signal(number, reason),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_label_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_label_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_set_parent_epic(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_set_parent_epic(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_unset_parent_epic(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_unset_parent_epic(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_reorder_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_reorder_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_move_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    move_story::handle_move_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_edit_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_edit_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_view_velocity(
    messages: Vec<crate::session::Message>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools::handle_view_velocity(|| async move { Ok(messages) }, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_view_board(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_view_board(&serde_json::Map::new(), refresh).await
}

pub(crate) async fn dispatch_test_get_story(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh.map(|r| r.store());
    tools::handle_get_story(&args, store.as_ref(), refresh).await
}

pub(crate) async fn dispatch_test_list_ready(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh
        .map(|r| r.store())
        .ok_or_else(|| "list_ready requires canonical store".to_string())?;
    tools::handle_list_ready(&store).await
}

pub(crate) async fn dispatch_test_list_epic_children(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh
        .map(|r| r.store())
        .ok_or_else(|| "list_epic_children requires canonical store".to_string())?;
    tools::handle_list_epic_children(&args, &store).await
}

pub(crate) async fn dispatch_test_dev_in_flight(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh
        .map(|r| r.store())
        .ok_or_else(|| "dev_in_flight requires canonical store".to_string())?;
    tools::handle_dev_in_flight(&store).await
}

pub(crate) async fn dispatch_test_view_story_timeline(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh.map(|r| r.store());
    tools::handle_view_story_timeline(&args, store.as_ref(), refresh).await
}

pub(crate) async fn dispatch_test_view_comments(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh.map(|r| r.store());
    tools::handle_view_comments(&args, store.as_ref(), refresh).await
}

pub(crate) async fn dispatch_test_view_backlog(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh
        .map(|r| r.store())
        .ok_or_else(|| "view_backlog requires canonical store".to_string())?;
    let stories = store.all_stories();
    let summary = tools::build_backlog_summary(&stories);
    serde_json::to_string(&summary).map_err(|e| format!("serialize summary: {e}"))
}

pub(crate) async fn dispatch_test_submit_verdict(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    tools::handle_submit_verdict(&args, agent_name, session).await
}
