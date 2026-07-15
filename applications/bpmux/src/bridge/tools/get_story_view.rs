use std::sync::Arc;

use crate::sync::RefreshDriver;
use product_management::iteration::section::{
    derive_iteration_member_section, substitute_member_stories_section,
};

pub(super) const NOT_HYDRATED_PLACEHOLDER: &str =
    "(body not yet hydrated; no refresh driver available to live-fetch.)";

pub(super) async fn render_iteration_detail(
    r: &Arc<dyn RefreshDriver>,
    iteration: &product_management::iteration::Iteration,
    select: &crate::store::Select,
) -> Result<String, String> {
    let id = iteration.id.value();
    let mut body = iteration.body.clone();
    if body.is_none() && r.fetch_body(id).await.is_ok() {
        body = select
            .select(|s| {
                product_management::iteration::get(
                    &s.iterations,
                    product_management::iteration::IterationId::new(id),
                )
            })
            .and_then(|it| it.body);
    }
    let body_hydrated = body.is_some();
    let mut body_out = body.unwrap_or_else(|| NOT_HYDRATED_PLACEHOLDER.to_string());
    if body_hydrated {
        let members: Vec<(u64, String)> = iteration
            .stories
            .iter()
            .filter_map(|n| {
                select
                    .select(|s| {
                        product_management::story::slice::get(
                            &s.stories,
                            product_management::story::StoryId::new(n.value()),
                        )
                    })
                    .filter(|s| !s.is_concluded())
                    .map(|s| (n.value(), s.title.clone()))
            })
            .collect();
        let members_refs: Vec<(u64, &str)> =
            members.iter().map(|(n, t)| (*n, t.as_str())).collect();
        let derived = derive_iteration_member_section(&members_refs);
        body_out = substitute_member_stories_section(&body_out, &derived);
    }
    let out = serde_json::json!({
        "id": id,
        "title": iteration.title,
        "body": body_out,
        "is_closed": iteration.complete,
        "is_current": iteration.is_current,
        "body_hydrated": body_hydrated,
    });
    serde_json::to_string_pretty(&out).map_err(|e| format!("serialize iteration: {e}"))
}

pub(super) async fn render_epic_detail(
    refresh: Option<&Arc<dyn RefreshDriver>>,
    epic: &product_management::epic::Epic,
    select: &crate::store::Select,
) -> Result<String, String> {
    let id = epic.id.value();
    let mut body = epic.body.clone();
    if body.is_none()
        && let Some(r) = refresh
        && r.fetch_body(id).await.is_ok()
    {
        body = select
            .select(|s| {
                product_management::epic::get(&s.epics, product_management::epic::EpicId::new(id))
            })
            .and_then(|e| e.body);
    }
    let body_hydrated = body.is_some();
    let body_out = body.unwrap_or_else(|| NOT_HYDRATED_PLACEHOLDER.to_string());
    let workflow_state = match refresh {
        Some(_) => {
            let stories = select.select(|s| product_management::story::slice::all(&s.stories));
            let children = product_management::epic::children_of(&stories, epic.id);
            let iterations = select.select(|s| product_management::iteration::all(&s.iterations));
            product_management::epic::derive_epic_state(&children, &iterations).as_str()
        }
        None => product_management::scheduling::ProjectedState::Icebox.as_str(),
    };
    let out = serde_json::json!({
        "id": id,
        "title": epic.title,
        "body": body_out,
        "workflow_state": workflow_state,
        "is_closed": epic.is_closed,
        "labels": [{ "name": "epic" }],
        "parent_epic": serde_json::Value::Null,
        "body_hydrated": body_hydrated,
        "gate_comments_hydrated": false,
        "latest_reject": serde_json::Value::Null,
    });
    serde_json::to_string_pretty(&out).map_err(|e| format!("serialize epic: {e}"))
}
