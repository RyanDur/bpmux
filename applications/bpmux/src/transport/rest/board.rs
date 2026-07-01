use std::sync::Arc;

pub(crate) fn ordered_board_stories(
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    let all = refresh.all_stories();
    let codec = crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
    let items: Vec<serde_json::Value> = all
        .iter()
        .map(|s| crate::product_management::board::adapters::json::story_to_board_json(&codec, s))
        .collect();
    let current_order: Vec<u64> = refresh
        .iterations()
        .iter()
        .find(|iteration| iteration.is_current)
        .map(|iteration| iteration.stories.iter().map(|id| id.value()).collect())
        .unwrap_or_default();
    crate::product_management::board::ready_order_label::sort_board_ready_by_membership(
        items,
        &current_order,
    )
}

pub(crate) fn board_stories(
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    ordered_board_stories(refresh)
}

pub(crate) fn board_epics(
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    let stories = refresh.all_stories();
    refresh
        .epics()
        .iter()
        .map(|epic| {
            let children: Vec<&crate::product_management::story::Story> = stories
                .iter()
                .filter(|s| s.parent_epic() == Some(epic.id))
                .collect();
            let derived = crate::product_management::epic::derive_epic_state(epic.state, &children);
            serde_json::json!({
                "id": epic.id.value(),
                "title": epic.title,
                "state": derived.as_str(),
                "gh_state": if epic.is_closed { "closed" } else { "open" },
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
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    use crate::product_management::board::view_model::IterationView;
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
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    crate::view::roadmap::roadmap_cards(refresh)
        .iter()
        .filter_map(|card| serde_json::to_value(card).ok())
        .collect()
}

pub(crate) fn board_outcomes(
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    use crate::product_management::board::view_model::OutcomeView;
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
