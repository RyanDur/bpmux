use std::sync::Arc;

use serde_json::Value;

use super::validation::known_tools;
use sync::SyncOrchestrator;

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

pub(in crate::rpc_routes) struct Ctx<'a> {
    pub(in crate::rpc_routes) agent: &'a str,
    pub(in crate::rpc_routes) role: &'a str,
    pub(in crate::rpc_routes) actor_id: Option<&'a str>,
    pub(in crate::rpc_routes) refresh_ref: Option<&'a Arc<SyncOrchestrator>>,
    pub(in crate::rpc_routes) hub: &'a Arc<hub::Hub>,
    pub(in crate::rpc_routes) dispatcher: &'a store::Dispatch,
    pub(in crate::rpc_routes) select: &'a store::Select,
    pub(in crate::rpc_routes) race_window: &'a sync::RaceWindowHandle,
    pub(in crate::rpc_routes) transitions:
        &'a std::sync::Arc<product_management::data_gateway::TransitionsDataGateway>,
}

pub(in crate::rpc_routes) enum Routed {
    Handled(Result<String, String>),
    Unhandled(serde_json::Map<String, Value>),
}

pub(super) fn effective_role(
    select: &store::Select,
    agent: &str,
    actor_id: Option<&str>,
) -> String {
    actor_id
        .and_then(|id| select.select(|state| store::routing::role_for(&state.routing, id)))
        .unwrap_or_else(|| bridge::tools_stories::role_from_name(agent).to_string())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn dispatch(
    refresh: &Arc<SyncOrchestrator>,
    tool: &str,
    caller: ToolCaller<'_>,
    args: serde_json::Map<String, Value>,
    hub: &Arc<hub::Hub>,
    dispatcher: &store::Dispatch,
    select: &store::Select,
    race_window: &sync::RaceWindowHandle,
    transitions: &std::sync::Arc<product_management::data_gateway::TransitionsDataGateway>,
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
        transitions,
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
