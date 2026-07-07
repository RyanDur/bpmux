use super::helpers::{ensure_story_in_canonical_store, require_purpose};

pub(crate) async fn handle_edit_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "edit story")?;
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let body = args.get("body").and_then(|v| v.as_str());
    let title = args.get("title").and_then(|v| v.as_str());
    let acceptor = match args.get("acceptor").and_then(|v| v.as_str()) {
        Some(raw) => Some(
            crate::product_management::story::Acceptor::parse(raw.trim()).ok_or_else(|| {
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
    let iteration = r.iteration(crate::product_management::iteration::IterationId::new(id));
    if let Some(b) = body
        && iteration.is_none()
    {
        let story = ensure_story_in_canonical_store(r, id).await?;
        if story.story_type() != crate::product_management::story::StoryKind::Chore {
            crate::bridge::atp_substantive_gate::verify_atp_pre_filing(b, None)
                .map_err(|e| e.as_json_string())?;
        }
    }
    let mut updated: Vec<&str> = Vec::new();
    if let Some(t) = title {
        r.update_title(id, t.to_string())
            .await
            .map_err(|e| format!("#{id} title update failed: {e}"))?;
        updated.push("title");
    }
    if let Some(b) = body {
        r.update_body(id, b.to_string())
            .await
            .map_err(|e| format!("#{id} body update failed: {e}"))?;
        updated.push("body");
    }
    if let Some(acceptor) = acceptor {
        ensure_story_in_canonical_store(r, id).await?;
        let meta = crate::product_management::story::events::EventMeta {
            actor: crate::routing::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::story::peer::PeerId::local(),
        };
        r.mutate_sync(crate::product_management::story::events::StoryEvent::new(
            meta,
            crate::product_management::story::StoryId::new(id),
            crate::product_management::story::events::StoryEventKind::AcceptorAssigned { acceptor },
        ))
        .map_err(|e| format!("#{id} acceptor enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{id} drain after acceptor re-set failed: {e}"))?;
        updated.push("acceptor");
    }
    if let Some(iteration) = iteration
        && (title.is_some() || body.is_some())
    {
        let new_title = title
            .map(str::to_string)
            .unwrap_or_else(|| iteration.title.clone());
        let new_body = match body {
            Some(b) => Some(b.to_string()),
            None => iteration.body.clone(),
        };
        dispatch.dispatch(crate::store::AppAction::Iteration(
            crate::product_management::iteration::IterationAction::Upserted(vec![
                crate::product_management::iteration::Iteration {
                    title: new_title,
                    body: new_body,
                    ..iteration
                },
            ]),
        ));
    }
    if (title.is_some() || body.is_some())
        && let Some(epic) = r.epic(crate::product_management::epic::EpicId::new(id))
    {
        let new_title = title
            .map(str::to_string)
            .unwrap_or_else(|| epic.title.clone());
        let new_body = match body {
            Some(b) => Some(b.to_string()),
            None => epic.body.clone(),
        };
        dispatch.dispatch(crate::store::AppAction::Epic(
            crate::product_management::epic::EpicAction::Upserted(vec![
                crate::product_management::epic::Epic {
                    title: new_title,
                    body: new_body,
                    ..epic
                },
            ]),
        ));
    }
    Ok(format!("#{id} {} updated", updated.join(" + ")))
}
