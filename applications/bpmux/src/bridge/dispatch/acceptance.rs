use std::sync::Arc;

use super::super::tools_stories;
use super::signal_broadcast::broadcast_client_msg_via_hub;

pub(crate) async fn dispatch_accept_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result = tools_stories::handle_accept_story(
        &args,
        agent_name,
        role,
        actor_id,
        refresh,
        select,
        Some(hub),
    )
    .await?;
    if let Some(id) = args.get("id").and_then(|v| v.as_u64()) {
        let signal_role = tools_stories::role_from_name(agent_name);
        broadcast_client_msg_via_hub(
            hub,
            agent_name,
            tools_stories::build_accept_signal(id, signal_role),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_fail_desk_check(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result =
        tools_stories::handle_fail_desk_check(&args, agent_name, role, actor_id, refresh, select)
            .await?;
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
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result = tools_stories::handle_pass_desk_check(
        &args,
        agent_name,
        role,
        actor_id,
        refresh,
        select,
        Some(hub),
    )
    .await?;
    if let Some(id) = args.get("id").and_then(|v| v.as_u64()) {
        let story_type = tools_stories::classify_for_pass_desk_check(id, refresh);
        let acceptor = refresh.and_then(|r| r.story(id)).and_then(|s| s.acceptor());
        for msg in tools_stories::build_pass_desk_check_signals(id, story_type, acceptor) {
            broadcast_client_msg_via_hub(hub, agent_name, msg);
        }
    }
    Ok(result)
}

pub(crate) async fn dispatch_reject_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
    hub: &Arc<crate::hub::Hub>,
) -> Result<String, String> {
    let result =
        tools_stories::handle_reject_story(&args, agent_name, role, actor_id, refresh, select)
            .await?;
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
