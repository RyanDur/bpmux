use tokio::sync::Mutex;

use super::super::tools::WsSink;
use super::helpers::{ensure_story_in_canonical_store, require_purpose, role_from_name};
use super::signals::broadcast_board_invalidate;

pub(crate) fn is_retired_iteration_label_axis(label: &str) -> bool {
    use crate::board::label_codec::LabelCodec;
    crate::board::adapters::gh_label_codec::GhLabelCodec
        .decode_iteration(label)
        .is_some()
}

pub(crate) fn retired_iteration_label_axis_error(label: &str) -> String {
    serde_json::json!({
        "kind": "tool-retired",
        "retired_axis": "iteration-label",
        "label": label,
        "successor": "add_story_to_iteration",
        "message": format!(
            "label_story refuses iteration-axis labels (#1402 retire). \
             Use add_story_to_iteration(iteration_number, story_number, position) \
             — iteration-membership is canonical-relation substrate per #1399, \
             not magic-string-parse labels. Label '{label}' parses as iteration-{{Theme}}.{{position}}."
        ),
    })
    .to_string()
}

pub(crate) async fn handle_label_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "label")?;
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
    if operation == "add" && is_retired_iteration_label_axis(label) {
        return Err(retired_iteration_label_axis_error(label));
    }
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
    require_purpose(agent_name, &["pm"], "set parent epic")?;
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

pub(crate) async fn handle_add_story_to_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let transition = crate::workflow::resolve_transition(
        workflow,
        "add-to-iteration",
        crate::workflow::GateClass::None,
    )
    .map_err(|e| e.as_json_string())?;
    crate::workflow::check_role_allowed(transition, actor_role).map_err(|e| e.as_json_string())?;
    let story_number = args
        .get("story_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'story_number' argument")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_number' argument")?;
    let position = args
        .get("position")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'position' argument")? as u32;
    if story_number == iteration_number {
        return Err(format!(
            "story #{story_number} cannot be added to itself as iteration-member"
        ));
    }
    let r = refresh.ok_or("add_story_to_iteration requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(r, story_number).await?;
    let parent = ensure_story_in_canonical_store(r, iteration_number).await?;
    if parent.story_type != crate::board::domain::StoryType::Iteration {
        return Err(format!(
            "story #{iteration_number} is not an Iteration (story_type={:?}); only Iteration stories can carry iteration-members",
            parent.story_type
        ));
    }
    let meta = crate::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    r.mutate_sync(crate::board::BoardEvent::IterationMembershipSet {
        meta,
        number: story_number,
        iteration_number,
        position,
    })
    .map_err(|e| format!("#{story_number} iteration-membership enqueue failed: {e}"))?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!(
        "#{story_number} added to iteration #{iteration_number} at position {position}"
    ))
}

pub(crate) async fn handle_unset_parent_epic(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "unset parent epic")?;
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

pub(crate) async fn handle_edit_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm", "owner"], "edit story")?;
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
    if let Some(b) = body {
        let story = ensure_story_in_canonical_store(r, number).await?;
        if story.story_type != crate::board::domain::StoryType::Chore {
            crate::bridge::atp_substantive_gate::verify_atp_pre_filing(b, None)
                .map_err(|e| e.as_json_string())?;
        }
    }
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
