use std::sync::Arc;

use super::super::tools_stories;
use super::signal_broadcast::broadcast_client_msg_via_hub;

pub(crate) async fn dispatch_deliver_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result =
        tools_stories::handle_deliver_story(&args, agent_name, role, actor_id, refresh, select)
            .await?;
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
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result =
        tools_stories::handle_start_story(&args, agent_name, role, actor_id, refresh, select)
            .await?;
    if let Some(id) = args.get("id").and_then(|v| v.as_u64()) {
        broadcast_client_msg_via_hub(hub, agent_name, tools_stories::build_start_signal(id));
    }
    Ok(result)
}

pub(crate) async fn dispatch_pass_stage(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result =
        tools_stories::handle_pass_stage(&args, agent_name, role, actor_id, refresh, select)
            .await?;
    broadcast_client_msg_via_hub(
        hub,
        agent_name,
        protocol::ClientMessage::Send {
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
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result =
        tools_stories::handle_fail_stage(&args, agent_name, role, actor_id, refresh, select)
            .await?;
    broadcast_client_msg_via_hub(
        hub,
        agent_name,
        protocol::ClientMessage::Send {
            text: result.clone(),
            to: None,
            addressee: None,
            message_type: Some("system".into()),
        },
    );
    Ok(result)
}
