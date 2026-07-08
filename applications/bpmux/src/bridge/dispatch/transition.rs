use std::sync::Arc;

use super::super::tools_stories;

pub(super) fn broadcast_client_msg_via_hub(
    hub: &Arc<crate::hub::Hub>,
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
        let from_short = crate::routing::short_name(agent_name);
        let hub_msg = crate::message::Message {
            from: format!("{from_short}:claude-code"),
            text,
            ts: chrono::Utc::now(),
            message_type: crate::message::MessageType::from(message_type.as_deref()),
            to,
            addressee,
            explicit_privacy,
            ..crate::message::Message::default()
        };
        if let Err(e) = hub.broadcast(&hub_msg) {
            crate::diag::log_diag(&format!("rpc: workflow-signal broadcast failed: {e}"));
        }
    }
}

pub(crate) async fn dispatch_deliver_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result = tools_stories::handle_deliver_story(&args, agent_name, refresh).await?;
    if let Some(id) = args.get("id").and_then(|v| v.as_u64()) {
        let title = refresh
            .and_then(|r| r.story(id))
            .map(|s| s.title)
            .unwrap_or_else(|| format!("#{id}"));
        for msg in tools_stories::build_delivery_signals(id, &title) {
            broadcast_client_msg_via_hub(hub, agent_name, msg);
        }
    }
    Ok(result)
}

pub(crate) async fn dispatch_start_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result = tools_stories::handle_start_story(&args, agent_name, refresh).await?;
    if let Some(id) = args.get("id").and_then(|v| v.as_u64()) {
        broadcast_client_msg_via_hub(hub, agent_name, tools_stories::build_start_signal(id));
    }
    Ok(result)
}

pub(crate) async fn dispatch_accept_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result = tools_stories::handle_accept_story(&args, agent_name, refresh, Some(hub)).await?;
    if let Some(id) = args.get("id").and_then(|v| v.as_u64()) {
        let role = tools_stories::role_from_name(agent_name);
        broadcast_client_msg_via_hub(
            hub,
            agent_name,
            tools_stories::build_accept_signal(id, role),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_fail_desk_check(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result = tools_stories::handle_fail_desk_check(&args, agent_name, refresh).await?;
    if let Some(id) = args.get("id").and_then(|v| v.as_u64()) {
        let reason = args
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("needs fixes");
        broadcast_client_msg_via_hub(
            hub,
            agent_name,
            tools_stories::build_fail_desk_check_signal(id, reason),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_pass_desk_check(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result =
        tools_stories::handle_pass_desk_check(&args, agent_name, refresh, Some(hub)).await?;
    if let Some(id) = args.get("id").and_then(|v| v.as_u64()) {
        let story_type = tools_stories::classify_for_pass_desk_check(id, refresh);
        let acceptor = refresh.and_then(|r| r.story(id)).and_then(|s| s.acceptor());
        for msg in tools_stories::build_pass_desk_check_signals(id, story_type, acceptor) {
            broadcast_client_msg_via_hub(hub, agent_name, msg);
        }
    }
    Ok(result)
}

pub(crate) async fn dispatch_pass_stage(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result = tools_stories::handle_pass_stage(&args, agent_name, refresh).await?;
    broadcast_client_msg_via_hub(
        hub,
        agent_name,
        crate::transport::protocol::ClientMessage::Send {
            text: result.clone(),
            to: None,
            addressee: None,
            message_type: Some("system".into()),
        },
    );
    Ok(result)
}

pub(crate) async fn dispatch_fail_stage(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result = tools_stories::handle_fail_stage(&args, agent_name, refresh).await?;
    broadcast_client_msg_via_hub(
        hub,
        agent_name,
        crate::transport::protocol::ClientMessage::Send {
            text: result.clone(),
            to: None,
            addressee: None,
            message_type: Some("system".into()),
        },
    );
    Ok(result)
}

pub(crate) async fn dispatch_reject_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result = tools_stories::handle_reject_story(&args, agent_name, refresh).await?;
    if let Some(id) = args.get("id").and_then(|v| v.as_u64()) {
        let reason = args
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("rejected");
        broadcast_client_msg_via_hub(
            hub,
            agent_name,
            tools_stories::build_reject_signal(id, reason),
        );
    }
    Ok(result)
}
