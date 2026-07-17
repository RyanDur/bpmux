use serde_json::Value;

use super::{Ctx, Routed};
use crate::bridge;

pub(super) async fn route(
    tool: &str,
    args: serde_json::Map<String, Value>,
    cx: &Ctx<'_>,
) -> Routed {
    let result = match tool {
        "create_iteration" => {
            bridge::dispatch_create_iteration(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.dispatcher,
            )
            .await
        }
        "close_iteration" => {
            bridge::dispatch_close_iteration(
                args,
                cx.role,
                cx.refresh_ref,
                cx.hub,
                cx.dispatcher,
                cx.select,
            )
            .await
        }
        "add_story_to_iteration" => {
            bridge::dispatch_add_story_to_iteration(
                args,
                cx.role,
                cx.refresh_ref,
                cx.dispatcher,
                cx.select,
            )
            .await
        }
        "designate_current_iteration" => {
            bridge::dispatch_designate_current_iteration(
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
        "close_pre_pickup_review" => {
            bridge::dispatch_close_pre_pickup_review(
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
        "dev_ok_iteration" => {
            bridge::dispatch_dev_ok_iteration(
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
        "remove_iteration_member" => {
            bridge::dispatch_remove_iteration_member(
                args,
                cx.role,
                cx.refresh_ref,
                cx.dispatcher,
                cx.select,
            )
            .await
        }
        "move_iteration_member" => {
            bridge::dispatch_move_iteration_member(
                args,
                cx.role,
                cx.refresh_ref,
                cx.dispatcher,
                cx.select,
            )
            .await
        }
        "rename_iteration" => {
            bridge::dispatch_rename_iteration(
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
