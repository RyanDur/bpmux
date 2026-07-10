use super::event_meta::meta_for;
use super::helpers::{compose_iteration_body, require_nonempty_string, role_from_name};
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::story::StoryId;

pub(crate) async fn handle_create_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
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
    let name = require_nonempty_string(
        args,
        "name",
        "name is required: short name for the iteration (e.g., 'Aquila', 'Pegasus'). Surfaces as the GH issue title 'Iteration {name}'.",
    )?;
    let r = refresh
        .ok_or("create_iteration requires refresh (RefreshDriver) — canonical store unavailable")?;
    let existing_open_iteration = r
        .iterations()
        .into_iter()
        .find(|it| !it.complete && it.title.eq_ignore_ascii_case(&format!("Iteration {name}")));
    if let Some(existing) = existing_open_iteration {
        return Err(format!(
            "iteration named '{name}' already open as #{n}; close it first or pick a different name (#1281 sister-discipline: duplicate-collision rejected at create-time)",
            n = existing.id.value()
        ));
    }
    let title = format!("Iteration {name}");
    let body = compose_iteration_body(name);
    let next_order = r
        .iterations()
        .into_iter()
        .filter(|it| !it.complete)
        .filter_map(|it| it.iteration_order)
        .max()
        .map_or(
            crate::product_management::iteration::label::DEFAULT_FIRST_POSITION,
            |m| m + crate::product_management::iteration::label::DEFAULT_STEP,
        );
    let id = r
        .create_story(title.clone(), body.clone(), vec!["iteration".to_string()])
        .await
        .map_err(|e| format!("create_iteration adapter call failed: {e}"))?;
    let meta = meta_for(agent_name, actor_id);
    let filed_by = meta.actor.clone();
    let filed_at = meta.ts;
    r.mutate_sync(crate::product_management::story::events::StoryEvent::new(
        meta.clone(),
        StoryId::new(id),
        crate::product_management::story::events::StoryEventKind::TagAdded {
            label: "iteration".to_string(),
        },
    ))
    .map_err(|e| format!("#{id} iteration label enqueue failed: {e}"))?;
    r.mutate_sync(crate::product_management::story::events::StoryEvent::new(
        meta,
        StoryId::new(id),
        crate::product_management::story::events::StoryEventKind::TagAdded {
            label: crate::integrations::github::gh_label_codec::GhLabelCodec
                .encode_iteration_order(next_order),
        },
    ))
    .map_err(|e| format!("#{id} iteration-order label enqueue failed: {e}"))?;
    dispatch.dispatch(crate::store::AppAction::Iteration(
        crate::product_management::iteration::IterationAction::Upserted(vec![Iteration {
            id: IterationId::new(id),
            title: title.clone(),
            body: Some(body),
            filed_at,
            filed_by,
            outcome: None,
            iteration_order: Some(next_order),
            stories: Vec::new(),
            review_closers: Vec::new(),
            dev_ok_by: Vec::new(),
            is_current: false,
            complete: false,
        }]),
    ));
    Ok(format!("iteration created: #{id} {title}"))
}
