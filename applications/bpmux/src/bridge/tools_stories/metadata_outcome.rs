use super::helpers::{ensure_story_in_canonical_store, require_purpose};

pub(crate) async fn handle_reprioritize_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "reprioritize an outcome")?;
    let outcome_number = args
        .get("outcome_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_number' argument")?;
    let to_raw = args
        .get("to")
        .and_then(serde_json::Value::as_str)
        .ok_or("reprioritize needs a target: pass --to <now|next|later>")?;
    let target = crate::board::domain::Horizon::from_cli(to_raw).ok_or_else(|| {
        format!(
            "invalid band '{to_raw}': must be {}",
            crate::board::domain::Horizon::CLI_VALUES
        )
    })?;

    let r = refresh.ok_or("reprioritize_outcome requires refresh (RefreshDriver)")?;
    let outcome = ensure_story_in_canonical_store(r, outcome_number).await?;
    if outcome.story_type != crate::board::domain::StoryType::Outcome {
        return Err(format!(
            "#{outcome_number} is not an Outcome (story_type={:?}); it may be a story, not an outcome — outcomes: bpmux roadmap",
            outcome.story_type
        ));
    }

    if outcome.horizon == Some(target) {
        return Ok(format!(
            "#{outcome_number} {} → {} (unchanged)",
            outcome.title,
            target.band_label()
        ));
    }

    let codec = crate::board::adapters::gh_label_codec::GhLabelCodec;
    let meta = crate::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    let target_label = codec.encode_horizon(target);
    for stale in outcome
        .raw_labels
        .iter()
        .filter(|l| codec.decode_horizon(l).is_some() && **l != target_label)
    {
        r.mutate_sync(crate::board::BoardEvent::TagRemoved {
            meta: meta.clone(),
            number: outcome_number,
            label: stale.clone(),
        })
        .map_err(|e| format!("#{outcome_number} prior-horizon removal enqueue failed: {e}"))?;
    }
    r.mutate_sync(crate::board::BoardEvent::TagAdded {
        meta,
        number: outcome_number,
        label: target_label,
    })
    .map_err(|e| format!("#{outcome_number} horizon label enqueue failed: {e}"))?;

    Ok(format!(
        "#{outcome_number} {} → {}",
        outcome.title,
        target.band_label()
    ))
}
