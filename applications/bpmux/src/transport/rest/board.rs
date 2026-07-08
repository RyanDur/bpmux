use std::sync::Arc;

pub(crate) fn ordered_board_stories(
    refresh: &Arc<dyn crate::sync::RefreshDriver>,
) -> Vec<serde_json::Value> {
    let all = refresh.all_stories();
    let iterations = refresh.iterations();
    let codec = crate::integrations::github::gh_label_codec::GhLabelCodec;
    let items: Vec<serde_json::Value> = all
        .iter()
        .map(|s| {
            let projected = crate::product_management::scheduling::projected_state(s, &iterations);
            let state = crate::board::wire_state(&projected, refresh.story_held_stage(s).as_ref());
            crate::view::board::story_to_board_json(&codec, s, &state)
        })
        .collect();
    let current_order: Vec<u64> =
        crate::product_management::iteration::label::current_iteration(&iterations)
            .map(|iteration| iteration.stories.iter().map(|id| id.value()).collect())
            .unwrap_or_default();
    sort_ready_by_membership(items, &current_order)
}

pub(crate) fn sort_ready_by_membership(
    stories: Vec<serde_json::Value>,
    current_order: &[u64],
) -> Vec<serde_json::Value> {
    let mut by_state: Vec<(String, Vec<serde_json::Value>)> = Vec::new();
    for s in stories {
        let state = s
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        match by_state.iter_mut().find(|(k, _)| k == &state) {
            Some((_, list)) => list.push(s),
            None => by_state.push((state, vec![s])),
        }
    }
    for (state, list) in by_state.iter_mut() {
        if state == crate::product_management::scheduling::ProjectedState::Ready.as_str() {
            list.sort_by_cached_key(|s| {
                let id = s.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                crate::product_management::scheduling::ready_membership_rank(id, current_order)
            });
        }
    }
    by_state.into_iter().flat_map(|(_, list)| list).collect()
}

pub(crate) fn board_stories(
    refresh: &Arc<dyn crate::sync::RefreshDriver>,
) -> Vec<serde_json::Value> {
    ordered_board_stories(refresh)
}

pub(crate) fn board_columns_json() -> Vec<serde_json::Value> {
    let workflow = crate::workflow::current_workflow();
    let middle = crate::workflow::board_middle_stage_ids(workflow);
    crate::board::board_columns(&middle)
        .iter()
        .map(|column| {
            let id = column_wire_id(column);
            serde_json::json!({ "id": id, "label": title_case(&id) })
        })
        .collect()
}

fn column_wire_id(column: &crate::board::Column) -> String {
    use crate::board::Column;
    match column {
        Column::Icebox => "icebox".to_string(),
        Column::Ready => "ready".to_string(),
        Column::Middle(stage) => stage.as_str().to_string(),
        Column::Acceptance => "acceptance".to_string(),
        Column::Accepted => "accepted".to_string(),
        Column::Retired => "retired".to_string(),
    }
}

fn title_case(id: &str) -> String {
    id.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

pub(crate) fn board_epics(refresh: &Arc<dyn crate::sync::RefreshDriver>) -> Vec<serde_json::Value> {
    let stories = refresh.all_stories();
    let iterations = refresh.iterations();
    refresh
        .epics()
        .iter()
        .map(|epic| {
            let children = crate::product_management::epic::children_of(&stories, epic.id);
            let derived =
                crate::product_management::epic::derive_epic_state(&children, &iterations);
            serde_json::json!({
                "id": epic.id.value(),
                "title": epic.title,
                "state": derived.as_str(),
                "closed": epic.is_closed,
                "labels": ["epic"],
                "assignees": [],
                "created_at": epic.filed_at.to_rfc3339(),
                "body": epic.body.clone().unwrap_or_default(),
                "parent_epic": serde_json::Value::Null,
            })
        })
        .collect()
}

pub(crate) fn board_iterations(
    refresh: &Arc<dyn crate::sync::RefreshDriver>,
) -> Vec<serde_json::Value> {
    use crate::view::board::IterationView;
    let outcomes = refresh.outcomes();
    let workflow = crate::workflow::current_workflow();
    refresh
        .iterations()
        .iter()
        .map(|iteration| {
            serde_json::to_value(IterationView::project(iteration, &outcomes, workflow))
                .unwrap_or_default()
        })
        .collect()
}

pub(crate) fn board_roadmap(
    refresh: &Arc<dyn crate::sync::RefreshDriver>,
) -> Vec<serde_json::Value> {
    crate::product_management::roadmap::selectors::roadmap_cards(
        &refresh.outcomes(),
        &refresh.iterations(),
        crate::workflow::current_workflow(),
    )
    .iter()
    .filter_map(|card| serde_json::to_value(card).ok())
    .collect()
}

pub(crate) fn board_outcomes(
    refresh: &Arc<dyn crate::sync::RefreshDriver>,
) -> Vec<serde_json::Value> {
    use crate::view::board::OutcomeView;
    let all_iterations = refresh.iterations();
    let workflow = crate::workflow::current_workflow();
    refresh
        .outcomes()
        .iter()
        .filter(|outcome| !outcome.is_retired())
        .map(|outcome| {
            serde_json::to_value(OutcomeView::project(outcome, &all_iterations, workflow))
                .unwrap_or_default()
        })
        .collect()
}
