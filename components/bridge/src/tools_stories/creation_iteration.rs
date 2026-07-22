use super::event_meta::meta_for;
use super::helpers::{compose_iteration_body, require_nonempty_string};
use product_management::iteration::IterationId;
use product_management::story::StoryId;

pub async fn handle_create_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<sync::SyncOrchestrator>>,
    dispatch: &store::Dispatch,
    select: &store::Select,
) -> Result<String, String> {
    let workflow = workflow::current_workflow();
    let pm = workflow.role_name_for_purpose("pm").unwrap_or("pm");
    let actor_role = role;
    let is_owner = workflow
        .owner_role_name()
        .is_some_and(|o| !o.is_empty() && actor_role == o);
    if actor_role != pm && !is_owner {
        return Err(
            "create_iteration is pm-lane. Iteration entities are PM-owned per Epic #1189."
                .to_string(),
        );
    }
    let name = require_nonempty_string(
        args,
        "name",
        "name is required: short name for the iteration (e.g., 'Aquila', 'Pegasus'). Surfaces as the GH issue title 'Iteration {name}'.",
    )?;
    let driver =
        refresh.ok_or("create_iteration requires refresh — canonical store unavailable")?;
    let open_iterations: Vec<product_management::iteration::Iteration> = select
        .select(|state| product_management::iteration::all(&state.iterations))
        .into_iter()
        .filter(|iteration| !iteration.complete)
        .collect();
    let existing_open_iteration = open_iterations.iter().find(|iteration| {
        iteration
            .title
            .eq_ignore_ascii_case(&format!("Iteration {name}"))
    });
    if let Some(existing) = existing_open_iteration {
        return Err(format!(
            "iteration named '{name}' already open as #{n}; close it first or pick a different name (#1281 sister-discipline: duplicate-collision rejected at create-time)",
            n = existing.id.value()
        ));
    }
    let title = format!("Iteration {name}");
    let body = compose_iteration_body(name);
    let next_order = open_iterations
        .iter()
        .filter_map(|iteration| iteration.iteration_order)
        .max()
        .map_or(
            product_management::iteration::label::DEFAULT_FIRST_POSITION,
            |m| m + product_management::iteration::label::DEFAULT_STEP,
        );
    let iteration_label = "iteration".to_string();
    let id = driver
        .create_story(&title, &body, &[iteration_label.as_str()])
        .await
        .map_err(|e| format!("create_iteration adapter call failed: {e}"))?;
    let meta = meta_for(agent_name, actor_id);
    let filed_by = meta.actor.clone();
    let filed_at = meta.ts;
    driver
        .mutate(product_management::story::events::StoryEvent::new(
            meta.clone(),
            StoryId::new(id),
            product_management::story::events::StoryEventKind::TagAdded {
                label: iteration_label.clone(),
            },
        ))
        .map_err(|e| format!("#{id} iteration label enqueue failed: {e}"))?;
    let order_label = source_codec::gh_label_codec::GhLabelCodec.encode_iteration_order(next_order);
    driver
        .mutate(product_management::story::events::StoryEvent::new(
            meta,
            StoryId::new(id),
            product_management::story::events::StoryEventKind::TagAdded {
                label: order_label.clone(),
            },
        ))
        .map_err(|e| format!("#{id} iteration-order label enqueue failed: {e}"))?;
    let labels = vec![iteration_label, order_label];
    let iteration = source_codec::mapper::assemble_iteration(
        IterationId::new(id),
        title.clone(),
        Some(body),
        filed_at,
        filed_by,
        &labels,
        false,
    );
    dispatch.dispatch(store::AppAction::Iteration(
        product_management::iteration::IterationAction::Upserted(vec![iteration]),
    ));
    Ok(format!("iteration created: #{id} {title}"))
}
