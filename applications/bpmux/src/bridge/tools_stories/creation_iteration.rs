use super::helpers::{compose_iteration_body, require_nonempty_string, role_from_name};

pub(crate) async fn handle_create_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
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
        s.story_type == crate::product_management::story::StoryType::Iteration
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
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(crate::product_management::board::BoardEvent::StoryCreated {
        meta: meta.clone(),
        number,
        title: title.clone(),
        body: body.clone(),
        initial_state: crate::product_management::story::StoryState::Icebox,
        story_type: crate::product_management::story::StoryType::Iteration,
    })
    .map_err(|e| format!("#{number} iteration created but enqueue failed: {e}"))?;
    r.mutate_sync(crate::product_management::board::BoardEvent::TagAdded {
        meta: meta.clone(),
        number,
        label: "iteration".to_string(),
    })
    .map_err(|e| format!("#{number} iteration label enqueue failed: {e}"))?;
    let next_order = r
        .all_stories()
        .into_iter()
        .filter(|s| {
            s.story_type == crate::product_management::story::StoryType::Iteration
                && !s.is_closed
                && s.number != number
        })
        .filter_map(|s| s.iteration_order)
        .max()
        .map_or(
            crate::product_management::iteration::label::DEFAULT_FIRST_POSITION,
            |m| m + crate::product_management::iteration::label::DEFAULT_STEP,
        );
    r.mutate_sync(crate::product_management::board::BoardEvent::TagAdded {
        meta,
        number,
        label: crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .encode_iteration_order(next_order),
    })
    .map_err(|e| format!("#{number} iteration-order label enqueue failed: {e}"))?;
    Ok(format!("iteration created: #{number} {title}"))
}
