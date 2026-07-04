use super::event_meta::meta_for;
use super::helpers::{ensure_iteration_in_canonical_store, require_purpose};
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::iteration::Iteration;
use crate::product_management::story::StoryId;
use crate::product_management::story::events::StoryEvent;
use crate::sync::RefreshDriver;
use std::sync::Arc;

pub(crate) async fn handle_reorder_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "reorder iterations")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_id' argument")?;
    let position = args
        .get("position")
        .and_then(serde_json::Value::as_u64)
        .ok_or("iteration reorder needs --to <P> (the new 1-indexed planning order)")?
        as u32;

    let r = refresh.ok_or("reorder_iteration requires refresh (RefreshDriver)")?;
    ensure_iteration_in_canonical_store(r, iteration_id).await?;

    let band_size = renumber_iteration_order(r, dispatch, agent_name, iteration_id, position)?;
    Ok(format!(
        "iteration #{iteration_id} reordered to planning position {position} of {band_size}"
    ))
}

fn renumber_iteration_order(
    r: &Arc<dyn RefreshDriver>,
    dispatch: &crate::store::Dispatch,
    agent_name: &str,
    moved: u64,
    requested_position: u32,
) -> Result<u32, String> {
    let mut actives: Vec<Iteration> = r
        .iterations()
        .into_iter()
        .filter(|iteration| !iteration.complete)
        .collect();
    actives.sort_by_key(|iteration| (iteration.iteration_order.unwrap_or(u32::MAX), iteration.id));

    let band_size = actives.len() as u32;
    let clamped = requested_position.clamp(1, band_size.max(1));
    let mut sequence: Vec<u64> = actives
        .iter()
        .map(|iteration| iteration.id.value())
        .filter(|number| *number != moved)
        .collect();
    let insert_index = (clamped as usize - 1).min(sequence.len());
    sequence.insert(insert_index, moved);

    let mut updates: Vec<Iteration> = Vec::new();
    for (index, number) in sequence.iter().enumerate() {
        let new_position = (index + 1) as u32;
        let Some(iteration) = actives
            .iter()
            .find(|iteration| iteration.id.value() == *number)
        else {
            continue;
        };
        if iteration.iteration_order == Some(new_position) {
            continue;
        }
        if let Some(old_position) = iteration.iteration_order {
            r.mutate_sync(StoryEvent::TagRemoved {
                meta: meta_for(agent_name),
                id: StoryId::new(*number),
                label: GhLabelCodec.encode_iteration_order(old_position),
            })
            .map_err(|e| format!("#{number} iteration-order clear enqueue failed: {e}"))?;
        }
        r.mutate_sync(StoryEvent::TagAdded {
            meta: meta_for(agent_name),
            id: StoryId::new(*number),
            label: GhLabelCodec.encode_iteration_order(new_position),
        })
        .map_err(|e| format!("#{number} iteration-order set enqueue failed: {e}"))?;
        updates.push(Iteration {
            iteration_order: Some(new_position),
            ..iteration.clone()
        });
    }
    if !updates.is_empty() {
        dispatch.dispatch(crate::store::AppAction::Iteration(
            crate::product_management::iteration::IterationAction::Upserted(updates),
        ));
    }
    Ok(band_size)
}

pub(crate) async fn handle_rename_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "edit an iteration name")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_id' argument")?;
    let name = args
        .get("name")
        .and_then(serde_json::Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .ok_or("iteration rename needs --name <N>")?;

    let r = refresh.ok_or("rename_iteration requires refresh (RefreshDriver)")?;
    let iteration = ensure_iteration_in_canonical_store(r, iteration_id).await?;
    let title = format!("Iteration {name}");
    r.update_title(iteration_id, title.clone())
        .await
        .map_err(|e| format!("#{iteration_id} name update failed: {e}"))?;
    dispatch.dispatch(crate::store::AppAction::Iteration(
        crate::product_management::iteration::IterationAction::Upserted(vec![Iteration {
            title,
            ..iteration
        }]),
    ));
    Ok(format!("iteration #{iteration_id} renamed to {name}"))
}
