use serde_json::Value;

use super::{Ctx, Routed};
use crate::bridge;

pub(super) async fn route(
    tool: &str,
    args: serde_json::Map<String, Value>,
    cx: &Ctx<'_>,
) -> Routed {
    let result = match tool {
        "start_story" => {
            bridge::dispatch_start_story(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.hub,
            )
            .await
        }
        "deliver_story" => {
            bridge::dispatch_deliver_story(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.hub,
            )
            .await
        }
        "pass_desk_check" => {
            bridge::dispatch_pass_desk_check(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.hub,
            )
            .await
        }
        "fail_desk_check" => {
            bridge::dispatch_fail_desk_check(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.hub,
            )
            .await
        }
        "pass_stage" => {
            bridge::dispatch_pass_stage(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.hub,
            )
            .await
        }
        "fail_stage" => {
            bridge::dispatch_fail_stage(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.hub,
            )
            .await
        }
        "accept_story" => {
            bridge::dispatch_accept_story(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.hub,
            )
            .await
        }
        "reject_story" => {
            bridge::dispatch_reject_story(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.hub,
            )
            .await
        }
        _ => return Routed::Unhandled(args),
    };
    Routed::Handled(result)
}
