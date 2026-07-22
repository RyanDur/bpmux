use serde_json::Value;

use super::{Ctx, Routed};
use bridge;

pub(super) async fn route(
    tool: &str,
    args: serde_json::Map<String, Value>,
    cx: &Ctx<'_>,
) -> Routed {
    let result = match tool {
        "create_outcome" => {
            bridge::dispatch_create_outcome(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.dispatcher,
            )
            .await
        }
        "map_iteration_to_outcome" => {
            bridge::dispatch_map_iteration_to_outcome(
                args,
                cx.role,
                cx.refresh_ref,
                cx.dispatcher,
                cx.select,
            )
            .await
        }
        "reprioritize_outcome" => {
            bridge::dispatch_reprioritize_outcome(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.dispatcher,
                cx.select,
            )
            .await
        }
        "edit_outcome" => {
            bridge::dispatch_edit_outcome(args, cx.role, cx.refresh_ref, cx.dispatcher, cx.select)
                .await
        }
        "retire_outcome" => {
            bridge::dispatch_retire_outcome(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.dispatcher,
                cx.select,
            )
            .await
        }
        "unmap_outcome_iteration" => {
            bridge::dispatch_unmap_outcome_iteration(
                args,
                cx.role,
                cx.refresh_ref,
                cx.dispatcher,
                cx.select,
            )
            .await
        }
        "reorder_outcome_iteration" => {
            bridge::dispatch_reorder_outcome_iteration(
                args,
                cx.role,
                cx.refresh_ref,
                cx.dispatcher,
                cx.select,
            )
            .await
        }
        _ => return Routed::Unhandled(args),
    };
    Routed::Handled(result)
}
