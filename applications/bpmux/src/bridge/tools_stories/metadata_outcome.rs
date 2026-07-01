use super::helpers::{ensure_outcome_in_canonical_store, require_purpose};
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::events::EventMeta;
use crate::product_management::board::{BoardEvent, PeerId, RefreshDriver};
use crate::product_management::outcome::{Outcome, OutcomeAction};
use crate::product_management::story::{Horizon, StoryId};
use std::sync::Arc;

pub(crate) async fn handle_reprioritize_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "reprioritize an outcome")?;
    let outcome_id = args
        .get("outcome_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_id' argument")?;
    let to_raw = args.get("to").and_then(serde_json::Value::as_str);
    let position = args
        .get("position")
        .and_then(serde_json::Value::as_u64)
        .map(|p| p as u32);
    if to_raw.is_none() && position.is_none() {
        return Err(
            "reprioritize needs a target: pass --to <now|next|later> and/or --position <P>"
                .to_string(),
        );
    }
    let requested_band = to_raw
        .map(|raw| {
            Horizon::from_cli(raw)
                .ok_or_else(|| format!("invalid band '{raw}': must be {}", Horizon::CLI_VALUES))
        })
        .transpose()?;

    let r = refresh.ok_or("reprioritize_outcome requires refresh (RefreshDriver)")?;
    let outcome = ensure_outcome_in_canonical_store(r, outcome_id).await?;

    let target_band = match requested_band {
        Some(band) => band,
        None => outcome.horizon().ok_or_else(|| {
            format!("#{outcome_id} has no band yet; pass --to <now|next|later> to set one")
        })?,
    };

    let codec = GhLabelCodec;
    let meta = || EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    };

    let band_changed = outcome.horizon() != Some(target_band);
    if band_changed {
        push_band_change(r, &codec, &meta, &outcome, target_band)?;
    }

    let moved = outcome.with_horizon(
        Some(target_band),
        if band_changed {
            None
        } else {
            outcome.horizon_order()
        },
    );

    let mut updates: Vec<Outcome> = Vec::new();
    let position_note = match position {
        Some(p) => reorder_within_band(r, &codec, &meta, &moved, target_band, p, &mut updates)?,
        None => {
            if band_changed {
                updates.push(moved);
            }
            String::new()
        }
    };

    if !band_changed && position.is_none() {
        return Ok(format!(
            "#{outcome_id} {} → {} (unchanged)",
            outcome.title,
            target_band.band_label()
        ));
    }
    if !updates.is_empty() {
        dispatch.dispatch(crate::store::AppAction::Outcome(OutcomeAction::Upserted(
            updates,
        )));
    }
    Ok(format!(
        "#{outcome_id} {} → {}{position_note}",
        outcome.title,
        target_band.band_label()
    ))
}

fn push_band_change(
    r: &Arc<dyn RefreshDriver>,
    codec: &GhLabelCodec,
    meta: &impl Fn() -> EventMeta,
    outcome: &Outcome,
    target_band: Horizon,
) -> Result<(), String> {
    let id = outcome.id.value();
    let target_label = codec.encode_horizon(target_band);
    let mut stale: Vec<String> = Vec::new();
    if let Some(current) = outcome.horizon() {
        let current_label = codec.encode_horizon(current);
        if current_label != target_label {
            stale.push(current_label);
        }
    }
    if let Some(order) = outcome.horizon_order() {
        stale.push(codec.encode_horizon_order(order));
    }
    for label in stale {
        r.mutate_sync(BoardEvent::TagRemoved {
            meta: meta(),
            id: StoryId::new(id),
            label,
        })
        .map_err(|e| format!("#{id} band-change label removal failed: {e}"))?;
    }
    r.mutate_sync(BoardEvent::TagAdded {
        meta: meta(),
        id: StoryId::new(id),
        label: target_label,
    })
    .map_err(|e| format!("#{id} horizon label enqueue failed: {e}"))
}

fn reorder_within_band(
    r: &Arc<dyn RefreshDriver>,
    codec: &GhLabelCodec,
    meta: &impl Fn() -> EventMeta,
    moved: &Outcome,
    band: Horizon,
    requested_position: u32,
    updates: &mut Vec<Outcome>,
) -> Result<String, String> {
    let moved_id = moved.id.value();
    let mut members: Vec<Outcome> = r
        .outcomes()
        .into_iter()
        .filter(|outcome| {
            outcome.horizon() == Some(band) && !outcome.is_retired() && outcome.id != moved.id
        })
        .collect();
    members.push(moved.clone());
    let mut ordered: Vec<&Outcome> = members.iter().collect();
    ordered.sort_by_key(|outcome| {
        (
            outcome.horizon_order().unwrap_or(u32::MAX),
            outcome.id.value(),
        )
    });

    let band_size = ordered.len() as u32;
    let clamped = requested_position.clamp(1, band_size.max(1));
    let mut sequence: Vec<u64> = ordered
        .iter()
        .map(|outcome| outcome.id.value())
        .filter(|n| *n != moved_id)
        .collect();
    let insert_index = (clamped as usize - 1).min(sequence.len());
    sequence.insert(insert_index, moved_id);

    for (index, id) in sequence.iter().enumerate() {
        let new_position = (index + 1) as u32;
        let member = members
            .iter()
            .find(|outcome| outcome.id.value() == *id)
            .ok_or_else(|| format!("#{id} vanished from band mid-reorder"))?;
        if member.horizon_order() == Some(new_position) {
            continue;
        }
        if let Some(order) = member.horizon_order() {
            r.mutate_sync(BoardEvent::TagRemoved {
                meta: meta(),
                id: StoryId::new(*id),
                label: codec.encode_horizon_order(order),
            })
            .map_err(|e| format!("#{id} prior-position removal failed: {e}"))?;
        }
        r.mutate_sync(BoardEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(*id),
            label: codec.encode_horizon_order(new_position),
        })
        .map_err(|e| format!("#{id} position label enqueue failed: {e}"))?;
        updates.push(member.with_horizon(Some(band), Some(new_position)));
    }
    Ok(format!(" (position {clamped} of {band_size})"))
}
