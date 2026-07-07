use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::product_management::outcome::Outcome;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::{Horizon, StoryId};
use crate::sync::RefreshDriver;
use std::sync::Arc;

pub(super) fn push_band_change(
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
        r.mutate_sync(StoryEvent::new(
            meta(),
            StoryId::new(id),
            StoryEventKind::TagRemoved { label },
        ))
        .map_err(|e| format!("#{id} band-change label removal failed: {e}"))?;
    }
    r.mutate_sync(StoryEvent::new(
        meta(),
        StoryId::new(id),
        StoryEventKind::TagAdded {
            label: target_label,
        },
    ))
    .map_err(|e| format!("#{id} horizon label enqueue failed: {e}"))
}

pub(super) fn reorder_within_band(
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
            r.mutate_sync(StoryEvent::new(
                meta(),
                StoryId::new(*id),
                StoryEventKind::TagRemoved {
                    label: codec.encode_horizon_order(order),
                },
            ))
            .map_err(|e| format!("#{id} prior-position removal failed: {e}"))?;
        }
        r.mutate_sync(StoryEvent::new(
            meta(),
            StoryId::new(*id),
            StoryEventKind::TagAdded {
                label: codec.encode_horizon_order(new_position),
            },
        ))
        .map_err(|e| format!("#{id} position label enqueue failed: {e}"))?;
        updates.push(member.with_horizon(Some(band), Some(new_position)));
    }
    Ok(format!(" (position {clamped} of {band_size})"))
}
