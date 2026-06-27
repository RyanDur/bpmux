use super::helpers::{ensure_story_in_canonical_store, require_purpose};
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::adapters::github::{
    horizon_from_labels, horizon_order_from_labels,
};
use crate::product_management::board::events::EventMeta;
use crate::product_management::board::{BoardEvent, PeerId, RefreshDriver};
use crate::product_management::story::{Horizon, Story, StoryType};
use std::sync::Arc;

pub(crate) async fn handle_reprioritize_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "reprioritize an outcome")?;
    let outcome_number = args
        .get("outcome_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_number' argument")?;
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
    let outcome = ensure_story_in_canonical_store(r, outcome_number).await?;
    let story_type = outcome.story_type();
    if story_type != StoryType::Outcome {
        return Err(format!(
            "#{outcome_number} is not an Outcome (story_type={:?}); it may be a story, not an outcome — outcomes: bpmux roadmap",
            story_type
        ));
    }

    let target_band = match requested_band {
        Some(band) => band,
        None => horizon_from_labels(&outcome.raw_labels).ok_or_else(|| {
            format!("#{outcome_number} has no band yet; pass --to <now|next|later> to set one")
        })?,
    };

    let codec = GhLabelCodec;
    let meta = || EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    };

    let band_changed = horizon_from_labels(&outcome.raw_labels) != Some(target_band);
    if band_changed {
        move_to_band(r, &codec, &meta, &outcome, target_band)?;
    }

    let position_note = match position {
        Some(p) => reorder_within_band(r, &codec, &meta, outcome_number, target_band, p)?,
        None => String::new(),
    };

    if !band_changed && position.is_none() {
        return Ok(format!(
            "#{outcome_number} {} → {} (unchanged)",
            outcome.title,
            target_band.band_label()
        ));
    }
    Ok(format!(
        "#{outcome_number} {} → {}{position_note}",
        outcome.title,
        target_band.band_label()
    ))
}

fn move_to_band(
    r: &Arc<dyn RefreshDriver>,
    codec: &GhLabelCodec,
    meta: &impl Fn() -> EventMeta,
    outcome: &Story,
    target_band: Horizon,
) -> Result<(), String> {
    let target_label = codec.encode_horizon(target_band);
    for stale in outcome
        .raw_labels
        .iter()
        .filter(|l| codec.decode_horizon(l).is_some() && **l != target_label)
        .chain(
            outcome
                .raw_labels
                .iter()
                .filter(|l| codec.decode_horizon_order(l).is_some()),
        )
    {
        r.mutate_sync(BoardEvent::TagRemoved {
            meta: meta(),
            number: outcome.number,
            label: stale.clone(),
        })
        .map_err(|e| format!("#{} band-change label removal failed: {e}", outcome.number))?;
    }
    r.mutate_sync(BoardEvent::TagAdded {
        meta: meta(),
        number: outcome.number,
        label: target_label,
    })
    .map_err(|e| format!("#{} horizon label enqueue failed: {e}", outcome.number))
}

fn reorder_within_band(
    r: &Arc<dyn RefreshDriver>,
    codec: &GhLabelCodec,
    meta: &impl Fn() -> EventMeta,
    outcome_number: u64,
    band: Horizon,
    requested_position: u32,
) -> Result<String, String> {
    let members: Vec<Story> = r
        .all_stories()
        .into_iter()
        .filter(|s| {
            s.story_type() == StoryType::Outcome && horizon_from_labels(&s.raw_labels) == Some(band)
        })
        .collect();
    let mut ordered: Vec<&Story> = members.iter().collect();
    ordered.sort_by_key(|s| {
        (
            horizon_order_from_labels(&s.raw_labels).unwrap_or(u32::MAX),
            s.number,
        )
    });

    let band_size = ordered.len() as u32;
    let clamped = requested_position.clamp(1, band_size.max(1));
    let mut sequence: Vec<u64> = ordered
        .iter()
        .map(|s| s.number)
        .filter(|n| *n != outcome_number)
        .collect();
    let insert_index = (clamped as usize - 1).min(sequence.len());
    sequence.insert(insert_index, outcome_number);

    for (index, number) in sequence.iter().enumerate() {
        let new_position = (index + 1) as u32;
        let member = members
            .iter()
            .find(|s| s.number == *number)
            .ok_or_else(|| format!("#{number} vanished from band mid-reorder"))?;
        if horizon_order_from_labels(&member.raw_labels) == Some(new_position) {
            continue;
        }
        for stale in member
            .raw_labels
            .iter()
            .filter(|l| codec.decode_horizon_order(l).is_some())
        {
            r.mutate_sync(BoardEvent::TagRemoved {
                meta: meta(),
                number: *number,
                label: stale.clone(),
            })
            .map_err(|e| format!("#{number} prior-position removal failed: {e}"))?;
        }
        r.mutate_sync(BoardEvent::TagAdded {
            meta: meta(),
            number: *number,
            label: codec.encode_horizon_order(new_position),
        })
        .map_err(|e| format!("#{number} position label enqueue failed: {e}"))?;
    }
    Ok(format!(" (position {clamped} of {band_size})"))
}
