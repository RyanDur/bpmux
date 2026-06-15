use super::helpers::{
    compose_iteration_body, compose_story_body, require_nonempty_string, role_from_name,
};

pub(crate) async fn handle_create_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("missing 'title' argument")?;
    let story_type = crate::board::domain::story_type_from_labels_or_title(&[], title);
    validate_actor_role_can_create(agent_name, story_type)?;
    let user_job = require_nonempty_string(
        args,
        "user_job",
        "user_job is required (2026-04-22 retro): 'When <situation>, <role> needs <affordance> because <constraint/goal>.' Cannot file a story without articulating the user-job it answers.",
    )?;
    let observation_falsifier = require_nonempty_string(
        args,
        "observation_falsifier",
        "observation_falsifier is required (2026-04-22 retro): 'I would reject this story if I observed X.' Frames acceptance as observation-that-fails rather than mechanism-that-passes.",
    )?;
    let acceptance_test_path = args
        .get("acceptance_test_path")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let author_body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let composed_body = compose_story_body(
        user_job,
        observation_falsifier,
        acceptance_test_path,
        author_body,
    );
    if story_type != crate::board::domain::StoryType::Chore {
        crate::bridge::atp_substantive_gate::verify_atp_pre_filing(
            &composed_body,
            acceptance_test_path,
        )
        .map_err(|e| e.as_json_string())?;
    }
    let parent_epic = args.get("parent_epic").and_then(|v| v.as_u64());
    let r = refresh.ok_or(
        "create_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n",
    )?;
    validate_parent_epic(r, parent_epic)?;
    let number = create_and_broadcast_story_events(
        r,
        agent_name,
        title,
        composed_body,
        story_type,
        parent_epic,
    )
    .await?;
    Ok(match parent_epic {
        Some(epic_number) => {
            format!("story created: #{number} {title} (under epic #{epic_number})")
        }
        None => format!("story created: #{number} {title}"),
    })
}

fn validate_actor_role_can_create(
    agent_name: &str,
    story_type: crate::board::domain::StoryType,
) -> Result<(), String> {
    let workflow = crate::workflow::current_workflow();
    let pm = workflow.role_id_for_purpose("pm").unwrap_or("pm");
    let dev = workflow.role_id_for_purpose("dev").unwrap_or("dev");
    let actor_role = role_from_name(agent_name);
    if let Some(owner) = workflow.owner_role_id()
        && !owner.is_empty()
        && actor_role == owner
    {
        return Ok(());
    }
    let is_chore = story_type == crate::board::domain::StoryType::Chore;
    if actor_role != pm && !(actor_role == dev && is_chore) {
        return Err(
            "create_story is pm-lane (dev may author [Chore]-class stories per chore-ownership \
             canon). If you have a pain to file, surface it in broadcast chat and pm will file \
             the story."
                .to_string(),
        );
    }
    Ok(())
}

fn validate_parent_epic(
    r: &std::sync::Arc<dyn crate::board::RefreshDriver>,
    parent_epic: Option<u64>,
) -> Result<(), String> {
    let Some(epic_number) = parent_epic else {
        return Ok(());
    };
    let parent = r.story(epic_number).ok_or_else(|| {
        format!(
            "epic #{epic_number} not found in canonical store — wait for next ingest tick or call refresh_story explicitly"
        )
    })?;
    if parent.story_type != crate::board::domain::StoryType::Epic {
        return Err(format!(
            "story #{epic_number} is not an Epic (story_type={:?}); only Epic stories can be parents",
            parent.story_type
        ));
    }
    Ok(())
}

async fn create_and_broadcast_story_events(
    r: &std::sync::Arc<dyn crate::board::RefreshDriver>,
    agent_name: &str,
    title: &str,
    composed_body: String,
    story_type: crate::board::domain::StoryType,
    parent_epic: Option<u64>,
) -> Result<u64, String> {
    let number = r
        .create_story(title.to_string(), composed_body.clone(), Vec::new())
        .await
        .map_err(|e| format!("create_story failed: {e}"))?;
    let meta = crate::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    r.mutate_sync(crate::board::BoardEvent::StoryCreated {
        meta: meta.clone(),
        number,
        title: title.to_string(),
        body: composed_body,
        initial_state: crate::board::domain::StoryState::Icebox,
        story_type,
    })
    .map_err(|e| format!("#{number} created but enqueue failed: {e}"))?;
    if let Some(epic_number) = parent_epic {
        r.mutate_sync(crate::board::BoardEvent::ParentEpicSet {
            meta,
            number,
            epic_number,
        })
        .map_err(|e| format!("#{number} parent_epic enqueue failed: {e}"))?;
    }
    Ok(number)
}

pub(crate) async fn handle_create_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let workflow = crate::workflow::current_workflow();
    let pm = workflow.role_id_for_purpose("pm").unwrap_or("pm");
    let actor_role = role_from_name(agent_name);
    let is_owner = workflow
        .owner_role_id()
        .is_some_and(|o| !o.is_empty() && actor_role == o);
    if actor_role != pm && !is_owner {
        return Err(
            "create_iteration is pm-lane. Iteration entities are PM-owned per Epic #1189."
                .to_string(),
        );
    }
    let theme_name = require_nonempty_string(
        args,
        "theme_name",
        "theme_name is required: short name for the iteration (e.g., 'Aquila', 'Pegasus'). Surfaces as the GH issue title 'Iteration {theme_name}'.",
    )?;
    let r = refresh
        .ok_or("create_iteration requires refresh (RefreshDriver) — canonical store unavailable")?;
    let existing_open_iteration = r.all_stories().into_iter().find(|s| {
        s.story_type == crate::board::domain::StoryType::Iteration
            && !s.is_closed
            && s.title
                .eq_ignore_ascii_case(&format!("Iteration {theme_name}"))
    });
    if let Some(existing) = existing_open_iteration {
        return Err(format!(
            "iteration named '{theme_name}' already open as #{n}; close it first or pick a different name (#1281 sister-discipline: duplicate-collision rejected at create-time)",
            n = existing.number
        ));
    }
    let title = format!("Iteration {theme_name}");
    let body = compose_iteration_body(theme_name);
    let number = r
        .create_story(title.clone(), body.clone(), vec!["iteration".to_string()])
        .await
        .map_err(|e| format!("create_iteration adapter call failed: {e}"))?;
    let meta = crate::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    r.mutate_sync(crate::board::BoardEvent::StoryCreated {
        meta: meta.clone(),
        number,
        title: title.clone(),
        body: body.clone(),
        initial_state: crate::board::domain::StoryState::Icebox,
        story_type: crate::board::domain::StoryType::Iteration,
    })
    .map_err(|e| format!("#{number} iteration created but enqueue failed: {e}"))?;
    r.mutate_sync(crate::board::BoardEvent::TagAdded {
        meta: meta.clone(),
        number,
        label: "iteration".to_string(),
    })
    .map_err(|e| format!("#{number} iteration label enqueue failed: {e}"))?;
    let next_order = r
        .all_stories()
        .into_iter()
        .filter(|s| {
            s.story_type == crate::board::domain::StoryType::Iteration
                && !s.is_closed
                && s.number != number
        })
        .filter_map(|s| s.iteration_order)
        .max()
        .map_or(crate::board::iteration_label::DEFAULT_FIRST_POSITION, |m| {
            m + crate::board::iteration_label::DEFAULT_STEP
        });
    r.mutate_sync(crate::board::BoardEvent::TagAdded {
        meta,
        number,
        label: crate::board::adapters::gh_label_codec::GhLabelCodec
            .encode_iteration_order(next_order),
    })
    .map_err(|e| format!("#{number} iteration-order label enqueue failed: {e}"))?;
    Ok(format!("iteration created: #{number} {title}"))
}

pub(crate) async fn handle_create_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let workflow = crate::workflow::current_workflow();
    let pm = workflow.role_id_for_purpose("pm").unwrap_or("pm");
    let actor_role = role_from_name(agent_name);
    let is_owner = workflow
        .owner_role_id()
        .is_some_and(|o| !o.is_empty() && actor_role == o);
    if actor_role != pm && !is_owner {
        return Err(
            "create_outcome is pm-lane. Outcomes are PM-owned roadmap entities (the why), distinct from board epics."
                .to_string(),
        );
    }
    let title = require_nonempty_string(
        args,
        "title",
        "title is required: the measurable behaviour-change the outcome pursues (the why).",
    )?
    .to_string();
    let horizon_raw = require_nonempty_string(
        args,
        "horizon",
        "horizon is required: one of now | next | later (sequence, not calendar).",
    )?;
    let horizon = match horizon_raw.to_ascii_lowercase().as_str() {
        "now" => crate::board::domain::Horizon::Now,
        "next" => crate::board::domain::Horizon::Next,
        "later" => crate::board::domain::Horizon::Later,
        other => {
            return Err(format!(
                "invalid horizon '{other}': must be now, next, or later (sequence, not calendar)"
            ));
        }
    };
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("## {title}\n"));
    let r = refresh
        .ok_or("create_outcome requires refresh (RefreshDriver) — canonical store unavailable")?;
    let number = r
        .create_story(title.clone(), body.clone(), vec!["outcome".to_string()])
        .await
        .map_err(|e| format!("create_outcome adapter call failed: {e}"))?;
    let meta = crate::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    r.mutate_sync(crate::board::BoardEvent::StoryCreated {
        meta: meta.clone(),
        number,
        title: title.clone(),
        body,
        initial_state: crate::board::domain::StoryState::Icebox,
        story_type: crate::board::domain::StoryType::Outcome,
    })
    .map_err(|e| format!("#{number} outcome created but enqueue failed: {e}"))?;
    r.mutate_sync(crate::board::BoardEvent::TagAdded {
        meta: meta.clone(),
        number,
        label: "outcome".to_string(),
    })
    .map_err(|e| format!("#{number} outcome label enqueue failed: {e}"))?;
    r.mutate_sync(crate::board::BoardEvent::TagAdded {
        meta,
        number,
        label: crate::board::adapters::gh_label_codec::GhLabelCodec.encode_horizon(horizon),
    })
    .map_err(|e| format!("#{number} outcome horizon label enqueue failed: {e}"))?;
    Ok(format!(
        "outcome created: #{number} {title} (horizon {})",
        horizon.as_str()
    ))
}
