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

pub(crate) fn board_iterations(
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    use crate::product_management::iteration::readiness::iteration_readiness;
    let outcomes = refresh.outcomes();
    let workflow = crate::workflow::current_workflow();
    refresh
        .iterations()
        .iter()
        .map(|iteration| {
            let id = iteration.id().saved_id().map(|id| id.value()).unwrap_or(0);
            let position = iteration.builds_outcome().and_then(|outcome_id| {
                outcomes
                    .iter()
                    .find(|o| {
                        o.id().saved_id().map(|id| {
                            crate::product_management::outcome::OutcomeId::new(id.value())
                        }) == Some(outcome_id)
                    })
                    .and_then(|o| {
                        o.position_of(crate::product_management::iteration::IterationId::new(id))
                    })
                    .map(|index| index + 1)
            });
            serde_json::json!({
                "id": id,
                "title": iteration.content.title,
                "iteration_order": iteration.iteration_order,
                "is_current": iteration.is_current,
                "complete": iteration.complete,
                "builds_outcome": iteration.builds_outcome(),
                "outcome_position": position,
                "stories": iteration.stories,
                "readiness": iteration_readiness(iteration, workflow),
            })
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
    use crate::product_management::iteration::readiness::{
        building_iterations, iteration_readiness, outcome_readiness,
    };
    let all_iterations = refresh.iterations();
    let workflow = crate::workflow::current_workflow();
    refresh
        .outcomes()
        .iter()
        .filter(|outcome| !outcome.is_retired())
        .map(|outcome| {
            let id = outcome.id().saved_id().map(|id| id.value()).unwrap_or(0);
            let iterations: Vec<serde_json::Value> = building_iterations(outcome, &all_iterations)
                .iter()
                .enumerate()
                .map(|(index, iteration)| {
                    serde_json::json!({
                        "id": iteration.id().saved_id().map(|id| id.value()).unwrap_or(0),
                        "name": iteration.content.title,
                        "position": index as u32 + 1,
                        "readiness": iteration_readiness(iteration, workflow),
                    })
                })
                .collect();
            serde_json::json!({
                "id": id,
                "title": outcome.content.title,
                "body": outcome.content.body.clone().unwrap_or_default(),
                "horizon": outcome.horizon().map(|horizon| horizon.as_str()),
                "horizon_order": outcome.horizon_order(),
                "outcome_readiness": outcome_readiness(outcome, &all_iterations, workflow),
                "iterations": iterations,
            })
        })
        .collect()
}
