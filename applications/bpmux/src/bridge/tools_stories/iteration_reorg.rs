use super::event_meta::meta_for;
use super::helpers::{ensure_iteration_in_canonical_store, require_purpose};
use crate::product_management::board::BoardEvent;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::iteration::Iteration;
use std::sync::Arc;

pub(crate) async fn handle_reorder_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "reorder iterations")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_number' argument")?;
    let position = args
        .get("position")
        .and_then(serde_json::Value::as_u64)
        .ok_or("iteration reorder needs --to <P> (the new 1-indexed planning order)")?
        as u32;

    let r = refresh.ok_or("reorder_iteration requires refresh (RefreshDriver)")?;
    ensure_iteration_in_canonical_store(r, iteration_number).await?;

    let band_size = renumber_iteration_order(r, agent_name, iteration_number, position)?;
    Ok(format!(
        "iteration #{iteration_number} reordered to planning position {position} of {band_size}"
    ))
}

fn renumber_iteration_order(
    r: &Arc<dyn RefreshDriver>,
    agent_name: &str,
    moved: u64,
    requested_position: u32,
) -> Result<u32, String> {
    let mut actives: Vec<Iteration> = r
        .iterations()
        .into_iter()
        .filter(|iteration| !iteration.complete)
        .collect();
    actives.sort_by_key(|iteration| {
        (
            iteration.iteration_order.unwrap_or(u32::MAX),
            iteration.id().saved_number().unwrap_or(0),
        )
    });

    let band_size = actives.len() as u32;
    let clamped = requested_position.clamp(1, band_size.max(1));
    let mut sequence: Vec<u64> = actives
        .iter()
        .filter_map(|iteration| iteration.id().saved_number())
        .filter(|number| *number != moved)
        .collect();
    let insert_index = (clamped as usize - 1).min(sequence.len());
    sequence.insert(insert_index, moved);

    for (index, number) in sequence.iter().enumerate() {
        let new_position = (index + 1) as u32;
        let Some(iteration) = actives
            .iter()
            .find(|iteration| iteration.id().saved_number() == Some(*number))
        else {
            continue;
        };
        if iteration.iteration_order == Some(new_position) {
            continue;
        }
        let raw_labels = r.story(*number).map(|s| s.raw_labels).unwrap_or_default();
        for stale in raw_labels
            .iter()
            .filter(|label| GhLabelCodec.decode_iteration_order(label).is_some())
        {
            r.mutate_sync(BoardEvent::TagRemoved {
                meta: meta_for(agent_name),
                number: *number,
                label: stale.clone(),
            })
            .map_err(|e| format!("#{number} iteration-order clear enqueue failed: {e}"))?;
        }
        r.mutate_sync(BoardEvent::TagAdded {
            meta: meta_for(agent_name),
            number: *number,
            label: GhLabelCodec.encode_iteration_order(new_position),
        })
        .map_err(|e| format!("#{number} iteration-order set enqueue failed: {e}"))?;
    }
    Ok(band_size)
}

pub(crate) async fn handle_rename_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "edit an iteration name")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_number' argument")?;
    let name = args
        .get("name")
        .and_then(serde_json::Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .ok_or("iteration rename needs --name <N>")?;

    let r = refresh.ok_or("rename_iteration requires refresh (RefreshDriver)")?;
    ensure_iteration_in_canonical_store(r, iteration_number).await?;
    r.update_title(iteration_number, format!("Iteration {name}"))
        .await
        .map_err(|e| format!("#{iteration_number} name update failed: {e}"))?;
    Ok(format!("iteration #{iteration_number} renamed to {name}"))
}
