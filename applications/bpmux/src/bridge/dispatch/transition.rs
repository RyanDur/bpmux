use std::sync::Arc;

use super::super::tools_stories;

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
            crate::diag::log_diag(&format!("rpc: workflow-signal broadcast failed: {e}"));
        }
    }
}

pub(crate) async fn dispatch_deliver_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let result = tools_stories::handle_deliver_story(&args, agent_name, refresh).await?;
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

pub(crate) async fn dispatch_start_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let result = tools_stories::handle_start_story(&args, agent_name, refresh).await?;
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        broadcast_client_msg_via_session(
            session,
            agent_name,
            tools_stories::build_start_signal(number),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_accept_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let result =
        tools_stories::handle_accept_story(&args, agent_name, refresh, Some(session)).await?;
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

pub(crate) async fn dispatch_fail_desk_check(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let result = tools_stories::handle_fail_desk_check(&args, agent_name, refresh).await?;
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

pub(crate) async fn dispatch_pass_desk_check(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let result =
        tools_stories::handle_pass_desk_check(&args, agent_name, refresh, Some(session)).await?;
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        let story_type = tools_stories::classify_for_pass_desk_check(number, refresh);
        let labels = refresh
            .and_then(|r| r.story(number))
            .map(|s| s.raw_labels)
            .unwrap_or_default();
        for msg in tools_stories::build_pass_desk_check_signals(number, story_type, &labels) {
            broadcast_client_msg_via_session(session, agent_name, msg);
        }
    }
    Ok(result)
}

pub(crate) async fn dispatch_reject_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let result = tools_stories::handle_reject_story(&args, agent_name, refresh).await?;
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
