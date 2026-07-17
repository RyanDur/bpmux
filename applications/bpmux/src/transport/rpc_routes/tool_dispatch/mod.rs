use std::sync::Arc;

use serde_json::Value;

use super::validation::known_tools;
use crate::hub;
use crate::sync::SyncOrchestrator;

mod iteration;
mod lifecycle;
mod outcome;
mod story;
mod views;

#[derive(Clone, Copy)]
pub(super) struct ToolCaller<'a> {
    pub(super) agent: &'a str,
    pub(super) role: &'a str,
    pub(super) actor_id: Option<&'a str>,
}

pub(in crate::transport::rpc_routes) struct Ctx<'a> {
    pub(in crate::transport::rpc_routes) agent: &'a str,
    pub(in crate::transport::rpc_routes) role: &'a str,
    pub(in crate::transport::rpc_routes) actor_id: Option<&'a str>,
    pub(in crate::transport::rpc_routes) refresh_ref: Option<&'a Arc<SyncOrchestrator>>,
    pub(in crate::transport::rpc_routes) hub: &'a Arc<hub::Hub>,
    pub(in crate::transport::rpc_routes) dispatcher: &'a crate::store::Dispatch,
    pub(in crate::transport::rpc_routes) select: &'a crate::store::Select,
    pub(in crate::transport::rpc_routes) race_window: &'a crate::sync::RaceWindowHandle,
}

pub(in crate::transport::rpc_routes) enum Routed {
    Handled(Result<String, String>),
    Unhandled(serde_json::Map<String, Value>),
}

pub(super) fn effective_role(
    select: &crate::store::Select,
    agent: &str,
    actor_id: Option<&str>,
) -> String {
    actor_id
        .and_then(|id| select.select(|state| crate::store::routing::role_for(&state.routing, id)))
        .unwrap_or_else(|| crate::bridge::tools_stories::role_from_name(agent).to_string())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn dispatch(
    refresh: &Arc<SyncOrchestrator>,
    tool: &str,
    caller: ToolCaller<'_>,
    args: serde_json::Map<String, Value>,
    hub: &Arc<hub::Hub>,
    dispatcher: &crate::store::Dispatch,
    select: &crate::store::Select,
    race_window: &crate::sync::RaceWindowHandle,
) -> Result<String, String> {
    let ToolCaller {
        agent,
        role,
        actor_id,
    } = caller;
    let refresh_ref: Option<&Arc<SyncOrchestrator>> = Some(refresh);
    let cx = Ctx {
        agent,
        role,
        actor_id,
        refresh_ref,
        hub,
        dispatcher,
        select,
        race_window,
    };
    let args = match lifecycle::route(tool, args, &cx).await {
        Routed::Handled(result) => return result,
        Routed::Unhandled(args) => args,
    };
    let args = match story::route(tool, args, &cx).await {
        Routed::Handled(result) => return result,
        Routed::Unhandled(args) => args,
    };
    let args = match iteration::route(tool, args, &cx).await {
        Routed::Handled(result) => return result,
        Routed::Unhandled(args) => args,
    };
    let args = match outcome::route(tool, args, &cx).await {
        Routed::Handled(result) => return result,
        Routed::Unhandled(args) => args,
    };
    match views::route(tool, args, &cx).await {
        Routed::Handled(result) => result,
        Routed::Unhandled(_) => Err(format!(
            "rpc: unknown tool '{tool}'; expected one of {:?}",
            known_tools()
        )),
    }
}
