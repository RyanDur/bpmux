use super::helpers::{ensure_story_in_canonical_store, require_purpose};
use crate::product_management::story::StoryId;

pub(crate) fn is_retired_iteration_label_axis(label: &str) -> bool {
    use crate::product_management::board::label_codec::LabelCodec;
    crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
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
             not magic-string-parse labels. Label '{label}' parses as iteration-{{Name}}.{{position}}."
        ),
    })
    .to_string()
}

pub(crate) async fn handle_label_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
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
        let meta = crate::product_management::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::board::PeerId::local(),
        };
        let (event, ok_msg) = match operation {
            "add" => (
                crate::product_management::board::BoardEvent::TagAdded {
                    meta,
                    id: StoryId::new(number),
                    label: label.to_string(),
                },
                format!("#{number} labeled '{label}'"),
            ),
            "remove" => (
                crate::product_management::board::BoardEvent::TagRemoved {
                    meta,
                    id: StoryId::new(number),
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
        return Ok(ok_msg);
    }
    Err(
        "label_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

pub(crate) async fn handle_comment_on_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
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
        let event = crate::product_management::board::BoardEvent::FreeCommentAppended {
            meta: crate::product_management::board::events::EventMeta {
                actor: crate::transport::ws::short_name(agent_name).to_string(),
                ts: chrono::Utc::now(),
                peer_id: crate::product_management::board::PeerId::local(),
            },
            id: StoryId::new(number),
            body: body.to_string(),
        };
        r.mutate_sync(event)
            .map_err(|e| format!("#{number} enqueue failed: {e}"))?;
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
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "edit story")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let body = args.get("body").and_then(|v| v.as_str());
    let title = args.get("title").and_then(|v| v.as_str());
    let acceptor = match args.get("acceptor").and_then(|v| v.as_str()) {
        Some(raw) => Some(
            super::acceptor_axis::AcceptorClass::parse(raw.trim()).ok_or_else(|| {
                format!("acceptor must be one of: team | owner | dev (got {raw:?})")
            })?,
        ),
        None => None,
    };
    if body.is_none() && title.is_none() && acceptor.is_none() {
        return Err(
            "edit_story requires at least one of 'body', 'title', or 'acceptor' (#1279, #1787)"
                .to_string(),
        );
    }
    let r = refresh
        .ok_or("edit_story requires refresh (RefreshDriver) — no canonical store available")?;
    if let Some(b) = body
        && r.iteration(number).is_none()
    {
        let story = ensure_story_in_canonical_store(r, number).await?;
        if story.kind() != Some(crate::product_management::story::StoryKind::Chore) {
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
    if let Some(acceptor) = acceptor {
        ensure_story_in_canonical_store(r, number).await?;
        let meta = crate::product_management::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::board::PeerId::local(),
        };
        let current_labels = r.story(number).map(|s| s.raw_labels).unwrap_or_default();
        for event in
            super::acceptor_axis::acceptor_label_events(acceptor, &current_labels, &meta, number)
        {
            r.mutate_sync(event)
                .map_err(|e| format!("#{number} acceptor-label enqueue failed: {e}"))?;
        }
        r.drain_pending()
            .await
            .map_err(|e| format!("#{number} drain after acceptor re-set failed: {e}"))?;
        updated.push("acceptor");
    }
    Ok(format!("#{number} {} updated", updated.join(" + ")))
}
