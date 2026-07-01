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
             Use add_story_to_iteration(iteration_id, story_id, position) \
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
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "label")?;
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
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
        ensure_story_in_canonical_store(r, id).await?;
        let meta = crate::product_management::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::board::PeerId::local(),
        };
        let (event, ok_msg) = match operation {
            "add" => (
                crate::product_management::board::StoryEvent::TagAdded {
                    meta,
                    id: StoryId::new(id),
                    label: label.to_string(),
                },
                format!("#{id} labeled '{label}'"),
            ),
            "remove" => (
                crate::product_management::board::StoryEvent::TagRemoved {
                    meta,
                    id: StoryId::new(id),
                    label: label.to_string(),
                },
                format!("#{id} unlabeled '{label}'"),
            ),
            other => {
                return Err(format!(
                    "invalid 'operation' argument '{other}': must be 'add' or 'remove'"
                ));
            }
        };
        r.mutate_sync(event)
            .map_err(|e| format!("#{id} enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{id} label push failed: {e}"))?;
        if r.epic(crate::product_management::epic::EpicId::new(id))
            .is_some()
            && let Some(story) = r.story(id)
        {
            dispatch.dispatch(crate::store::AppAction::Epic(
                crate::product_management::epic::EpicAction::Upserted(vec![
                    crate::product_management::board::adapters::github::decode_epic_from_story(
                        &story,
                    ),
                ]),
            ));
        }
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
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or("missing 'body' argument")?;
    if body.trim().is_empty() {
        return Err("'body' must not be empty".to_string());
    }
    if let Some(r) = refresh {
        ensure_story_in_canonical_store(r, id).await?;
        let event = crate::product_management::board::StoryEvent::FreeCommentAppended {
            meta: crate::product_management::board::events::EventMeta {
                actor: crate::transport::ws::short_name(agent_name).to_string(),
                ts: chrono::Utc::now(),
                peer_id: crate::product_management::board::PeerId::local(),
            },
            id: StoryId::new(id),
            body: body.to_string(),
        };
        r.mutate_sync(event)
            .map_err(|e| format!("#{id} enqueue failed: {e}"))?;
        return Ok(format!("#{id} commented"));
    }
    Err(
        "comment_on_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}
