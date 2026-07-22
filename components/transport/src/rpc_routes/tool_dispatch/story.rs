use serde_json::Value;

use super::{Ctx, Routed};
use bridge;

pub(super) async fn route(
    tool: &str,
    args: serde_json::Map<String, Value>,
    cx: &Ctx<'_>,
) -> Routed {
    let result = match tool {
        "close_story" => {
            bridge::dispatch_close_story(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.select,
            )
            .await
        }
        "create_story" => {
            bridge::dispatch_create_story(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.select,
                cx.dispatcher,
            )
            .await
        }
        "comment_on_story" => {
            bridge::dispatch_comment_on_story(
                args,
                cx.agent,
                cx.actor_id,
                cx.refresh_ref,
                cx.select,
            )
            .await
        }
        "label_story" => {
            bridge::dispatch_label_story(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.select,
            )
            .await
        }
        "set_parent_epic" => {
            bridge::dispatch_set_parent_epic(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.select,
            )
            .await
        }
        "unset_parent_epic" => {
            bridge::dispatch_unset_parent_epic(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.select,
            )
            .await
        }
        "reorder_story" => bridge::dispatch_reorder_story(args, cx.agent, cx.refresh_ref).await,
        "move_story" => {
            bridge::dispatch_move_story(
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
        "edit_story" => {
            bridge::dispatch_edit_story(
                args,
                cx.agent,
                cx.role,
                cx.actor_id,
                cx.refresh_ref,
                cx.select,
                cx.dispatcher,
            )
            .await
        }
        _ => return Routed::Unhandled(args),
    };
    Routed::Handled(result)
}
