//! Metadata handlers: label add/remove, parent-epic set/unset,
//! free-form comments, and body/title edits. Each mutates the
//! canonical store via `RefreshDriver` and broadcasts a board-
//! invalidate so peer renderers refresh.

use tokio::sync::Mutex;

use super::super::tools::WsSink;
use super::helpers::{ensure_story_in_canonical_store, require_role};
use super::signals::broadcast_board_invalidate;

/// label_story accepts an `operation` arg of "add" (default) or
/// "remove" (#392). The remove path closes the earlier bypass —
/// before this story, label removal required raw `gh issue edit
/// --remove-label`, which mutated GitHub without firing the
/// bridge's BOARD_INVALIDATE broadcast and stranded peer boards on
/// stale state. Both paths now broadcast on success; errors bubble
/// up without broadcasting so peers aren't asked to refresh against
/// a no-op. Remove is idempotent — the tracker maps GitHub's 404
/// ("label not attached") to Ok so callers can safely
/// remove-without-checking.
pub(crate) async fn handle_label_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "label")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let label = args
        .get("label")
        .and_then(|v| v.as_str())
        .ok_or("missing 'label' argument")?;
    let operation = args
        .get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or("add");
    if let Some(r) = refresh {
        ensure_story_in_canonical_store(r, number).await?;
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        let (event, ok_msg) = match operation {
            "add" => (
                crate::board::BoardEvent::TagAdded {
                    meta,
                    number,
                    label: label.to_string(),
                },
                format!("#{number} labeled '{label}'"),
            ),
            "remove" => (
                crate::board::BoardEvent::TagRemoved {
                    meta,
                    number,
                    label: label.to_string(),
                },
                format!("#{number} unlabeled '{label}'"),
            ),
            other => {
                return Err(format!(
                    "invalid 'operation' argument '{other}': must be 'add' or 'remove'"
                ));
            }
        };
        r.mutate_sync(event)
            .map_err(|e| format!("#{number} enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{number} label push failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(ok_msg);
    }
    Err(
        "label_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

pub(crate) async fn handle_set_parent_epic(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "set parent epic")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let epic_number = args
        .get("epic_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'epic_number' argument")?;
    if number == epic_number {
        return Err(format!("story #{number} cannot be its own parent epic"));
    }
    let r = refresh.ok_or("set_parent_epic requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(r, number).await?;
    let parent = ensure_story_in_canonical_store(r, epic_number).await?;
    if parent.story_type != crate::board::domain::StoryType::Epic {
        return Err(format!(
            "story #{epic_number} is not an Epic (story_type={:?}); only Epic stories can be parents",
            parent.story_type
        ));
    }
    let meta = crate::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    r.mutate_sync(crate::board::BoardEvent::ParentEpicSet {
        meta,
        number,
        epic_number,
    })
    .map_err(|e| format!("#{number} parent_epic enqueue failed: {e}"))?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} grouped under epic #{epic_number}"))
}

pub(crate) async fn handle_unset_parent_epic(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "unset parent epic")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let r = refresh.ok_or("unset_parent_epic requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(r, number).await?;
    let meta = crate::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    r.mutate_sync(crate::board::BoardEvent::ParentEpicUnset { meta, number })
        .map_err(|e| format!("#{number} parent_epic-unset enqueue failed: {e}"))?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} ungrouped from parent epic"))
}

/// Add a comment to a story — any agent. Durable issue-audit-trail
/// (visible to any future reader of the issue; distinct from bpmux
/// chat which is session-scoped). Routes through the RefreshDriver
/// per #451; reject_story invokes the same path.
pub(crate) async fn handle_comment_on_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or("missing 'body' argument")?;
    if body.trim().is_empty() {
        return Err("'body' must not be empty".to_string());
    }
    if let Some(r) = refresh {
        ensure_story_in_canonical_store(r, number).await?;
        let event = crate::board::BoardEvent::FreeCommentAppended {
            meta: crate::board::events::EventMeta {
                actor: crate::transport::ws::short_name(agent_name).to_string(),
                ts: chrono::Utc::now(),
                peer_id: crate::board::PeerId::local(),
            },
            number,
            body: body.to_string(),
        };
        r.mutate_sync(event)
            .map_err(|e| format!("#{number} enqueue failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(format!("#{number} commented"));
    }
    Err(
        "comment_on_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

/// Replace a story's body or title via the canonical store (#530).
/// Closes the gh-bypass class observed during ATP-corrections — edits
/// surface in the BoardStore + WS broadcast so every observer sees
/// the change without a separate refresh cycle. At least one of
/// `body` or `title` must be supplied (#1279).
pub(crate) async fn handle_edit_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm", "ryan"], "edit story")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let body = args.get("body").and_then(|v| v.as_str());
    let title = args.get("title").and_then(|v| v.as_str());
    if body.is_none() && title.is_none() {
        return Err("edit_story requires at least one of 'body' or 'title' (#1279)".to_string());
    }
    let r = refresh
        .ok_or("edit_story requires refresh (RefreshDriver) — no canonical store available")?;
    let mut updated: Vec<&str> = Vec::new();
    if let Some(t) = title {
        r.update_title(number, t.to_string())
            .await
            .map_err(|e| format!("#{number} title update failed: {e}"))?;
        updated.push("title");
    }
    if let Some(b) = body {
        r.update_body(number, b.to_string())
            .await
            .map_err(|e| format!("#{number} body update failed: {e}"))?;
        updated.push("body");
    }
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} {} updated", updated.join(" + ")))
}
