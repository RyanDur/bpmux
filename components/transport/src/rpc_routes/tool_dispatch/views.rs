use serde_json::Value;

use super::{Ctx, Routed};
use bridge;

pub(super) async fn route(
    tool: &str,
    args: serde_json::Map<String, Value>,
    cx: &Ctx<'_>,
) -> Routed {
    let result = match tool {
        "view_board" => bridge::dispatch_view_board(args, cx.refresh_ref, cx.select).await,
        "get_story" => bridge::dispatch_get_story(args, cx.refresh_ref, cx.select).await,
        "list_ready" => bridge::dispatch_list_ready(cx.select).await,
        "list_epic_children" => bridge::dispatch_list_epic_children(args, cx.select).await,
        "dev_in_flight" => bridge::dispatch_dev_in_flight(cx.select).await,
        "view_story_timeline" => {
            bridge::dispatch_view_story_timeline(args, cx.refresh_ref, cx.select).await
        }
        "view_comments" => bridge::dispatch_view_comments(args, cx.select).await,
        "view_backlog" => bridge::dispatch_view_backlog(cx.select).await,
        "submit_verdict" => {
            bridge::dispatch_submit_verdict(
                args,
                cx.agent,
                cx.role,
                cx.hub,
                cx.dispatcher,
                cx.select,
            )
            .await
        }
        "view_verdicts" => bridge::dispatch_view_verdicts(args, cx.select).await,
        "view_velocity" => serde_json::to_string_pretty(&crate::rest::metrics::assemble_metrics(
            cx.select,
            cx.transitions,
        ))
        .map_err(|e| format!("serialize view_velocity: {e}")),
        "view_story_commits" => bridge::dispatch_view_story_commits(args).await,
        "view_commit_diff" => bridge::dispatch_view_commit_diff(args).await,
        _ => return Routed::Unhandled(args),
    };
    Routed::Handled(result)
}
