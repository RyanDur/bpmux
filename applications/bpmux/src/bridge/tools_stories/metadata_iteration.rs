use super::helpers::{ensure_story_in_canonical_store, require_purpose, role_from_name};

pub(crate) async fn handle_add_story_to_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let transition = crate::workflow::resolve_transition(
        workflow,
        "add-to-iteration",
        crate::workflow::GateClass::None,
    )
    .map_err(|e| e.as_json_string())?;
    crate::workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_id().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;
    let story_number = args
        .get("story_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'story_number' argument")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_number' argument")?;
    let position = args
        .get("position")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'position' argument")? as u32;
    if story_number == iteration_number {
        return Err(format!(
            "story #{story_number} cannot be added to itself as iteration-member"
        ));
    }
    let r = refresh.ok_or("add_story_to_iteration requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(r, story_number).await?;
    let parent = ensure_story_in_canonical_store(r, iteration_number).await?;
    let story_type =
        crate::product_management::board::adapters::github::story_type_from_labels_or_title(
            &parent.raw_labels,
            &parent.title,
        );
    if story_type != crate::product_management::story::StoryType::Iteration {
        return Err(format!(
            "story #{iteration_number} is not an Iteration (story_type={:?}); only Iteration stories can carry iteration-members",
            story_type
        ));
    }
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(
        crate::product_management::board::BoardEvent::IterationMembershipSet {
            meta,
            number: story_number,
            iteration_number,
            position,
        },
    )
    .map_err(|e| format!("#{story_number} iteration-membership enqueue failed: {e}"))?;
    Ok(format!(
        "#{story_number} added to iteration #{iteration_number} at position {position}"
    ))
}

pub(crate) async fn handle_map_iteration_to_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "map an iteration to an outcome")?;
    let outcome_number = args
        .get("outcome_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'outcome_number' argument")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_number' argument")?;
    let position = args
        .get("position")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'position' argument")? as u32;
    let r = refresh.ok_or("map_iteration_to_outcome requires refresh (RefreshDriver)")?;
    let iteration = ensure_story_in_canonical_store(r, iteration_number).await?;
    let story_type =
        crate::product_management::board::adapters::github::story_type_from_labels_or_title(
            &iteration.raw_labels,
            &iteration.title,
        );
    if story_type != crate::product_management::story::StoryType::Iteration {
        return Err(format!(
            "story #{iteration_number} is not an Iteration (story_type={:?}); only iterations build outcomes",
            story_type
        ));
    }
    let outcome = ensure_story_in_canonical_store(r, outcome_number).await?;
    let story_type =
        crate::product_management::board::adapters::github::story_type_from_labels_or_title(
            &outcome.raw_labels,
            &outcome.title,
        );
    if story_type != crate::product_management::story::StoryType::Outcome {
        return Err(format!(
            "story #{outcome_number} is not an Outcome (story_type={:?}); iterations map only to outcomes",
            story_type
        ));
    }
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(
        crate::product_management::board::BoardEvent::OutcomeMembershipSet {
            meta,
            number: iteration_number,
            outcome_number,
            position,
        },
    )
    .map_err(|e| format!("#{iteration_number} outcome-membership enqueue failed: {e}"))?;
    Ok(format!(
        "iteration #{iteration_number} maps to outcome #{outcome_number} at position {position}"
    ))
}
