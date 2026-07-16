use super::event_meta::meta_for;
use super::helpers::{ensure_outcome_in_canonical_store, require_purpose};
use super::metadata_outcome_bands::{push_band_change, reorder_within_band};
use crate::sync::RefreshDriver;
use product_management::outcome::{Outcome, OutcomeAction};
use product_management::story::Horizon;
use source_codec::gh_label_codec::GhLabelCodec;
use std::sync::Arc;

pub(crate) async fn handle_reprioritize_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<dyn RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    require_purpose(role, &["pm"], "reprioritize an outcome")?;
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

    let driver = refresh.ok_or("reprioritize_outcome requires refresh (RefreshDriver)")?;
    let outcome = ensure_outcome_in_canonical_store(driver, select, outcome_id).await?;

    let target_band = match requested_band {
        Some(band) => band,
        None => outcome.horizon().ok_or_else(|| {
            format!("#{outcome_id} has no band yet; pass --to <now|next|later> to set one")
        })?,
    };

    let codec = GhLabelCodec;
    let meta = || meta_for(agent_name, actor_id);

    let band_changed = outcome.horizon() != Some(target_band);
    if band_changed {
        push_band_change(driver, &codec, &meta, &outcome, target_band)?;
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
        Some(p) => {
            reorder_within_band(driver, &codec, &meta, &moved, target_band, p, &mut updates)?
        }
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
