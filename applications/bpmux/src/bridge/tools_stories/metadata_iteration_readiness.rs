use super::helpers::{
    ensure_story_in_canonical_store, require_purpose, require_purpose_any, role_from_name,
};

pub(crate) async fn handle_designate_current_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "designate current iteration")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_number' argument")?;
    let r = refresh.ok_or("designate_current_iteration requires refresh (RefreshDriver)")?;
    let target = ensure_story_in_canonical_store(r, iteration_number).await?;
    if target.story_type != crate::product_management::story::StoryType::Iteration {
        return Err(format!(
            "story #{iteration_number} is not an Iteration (story_type={:?}); only Iteration stories can be designated current",
            target.story_type
        ));
    }
    if target.is_closed {
        return Err(format!(
            "iteration #{iteration_number} is closed; a closed iteration cannot be the current iteration"
        ));
    }
    let meta = || crate::product_management::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    let prior: Vec<u64> = r
        .all_stories()
        .into_iter()
        .filter(|s| {
            crate::product_management::board::adapters::github::is_current_from_labels(
                &s.raw_labels,
            ) && s.number != iteration_number
        })
        .map(|s| s.number)
        .collect();
    for n in prior {
        r.mutate_sync(
            crate::product_management::board::BoardEvent::CurrentIterationCleared {
                meta: meta(),
                number: n,
            },
        )
        .map_err(|e| format!("#{n} current-designation-clear enqueue failed: {e}"))?;
    }
    r.mutate_sync(
        crate::product_management::board::BoardEvent::CurrentIterationDesignated {
            meta: meta(),
            number: iteration_number,
        },
    )
    .map_err(|e| format!("#{iteration_number} current-designation enqueue failed: {e}"))?;
    Ok(format!("iteration #{iteration_number} designated current"))
}

pub(crate) async fn handle_close_pre_pickup_review(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose_any(
        agent_name,
        crate::product_management::iteration::readiness::ITERATION_REVIEWER_PURPOSE,
        "close the pre-pickup review",
    )?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_number' argument")?;
    let r = refresh.ok_or("close_pre_pickup_review requires refresh (RefreshDriver)")?;
    let target = ensure_story_in_canonical_store(r, iteration_number).await?;
    if target.story_type != crate::product_management::story::StoryType::Iteration {
        return Err(format!(
            "story #{iteration_number} is not an Iteration (story_type={:?}); only an Iteration's pre-pickup review can be closed",
            target.story_type
        ));
    }
    let role = role_from_name(agent_name).to_string();
    let label = crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
        .encode_iteration_review_closed(&role);
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(crate::product_management::board::BoardEvent::TagAdded {
        meta,
        number: iteration_number,
        label,
    })
    .map_err(|e| format!("#{iteration_number} review-close enqueue failed: {e}"))?;
    r.drain_pending()
        .await
        .map_err(|e| format!("#{iteration_number} review-close push failed: {e}"))?;
    Ok(format!(
        "pre-pickup review for iteration #{iteration_number} closed by {role}"
    ))
}

pub(crate) async fn handle_dev_ok_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose_any(
        agent_name,
        crate::product_management::iteration::readiness::ITERATION_BUILDER_PURPOSE,
        "record dev-OK for the iteration",
    )?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_number' argument")?;
    let r = refresh.ok_or("dev_ok_iteration requires refresh (RefreshDriver)")?;
    let target = ensure_story_in_canonical_store(r, iteration_number).await?;
    if target.story_type != crate::product_management::story::StoryType::Iteration {
        return Err(format!(
            "story #{iteration_number} is not an Iteration (story_type={:?}); only an Iteration can be dev-OK'd",
            target.story_type
        ));
    }
    let role = role_from_name(agent_name).to_string();
    let label = crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
        .encode_iteration_dev_ok(&role);
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(crate::product_management::board::BoardEvent::TagAdded {
        meta,
        number: iteration_number,
        label,
    })
    .map_err(|e| format!("#{iteration_number} dev-OK enqueue failed: {e}"))?;
    r.drain_pending()
        .await
        .map_err(|e| format!("#{iteration_number} dev-OK push failed: {e}"))?;
    Ok(format!("iteration #{iteration_number} dev-OK'd by {role}"))
}
