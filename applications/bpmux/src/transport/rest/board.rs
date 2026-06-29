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
    crate::product_management::board::ready_order_label::sort_board_ready_by_membership(items)
}

pub(crate) fn board_iterations(
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    use crate::product_management::iteration::readiness::iteration_readiness;
    let all = refresh.all_stories();
    let outcomes = refresh.outcomes();
    let workflow = crate::workflow::current_workflow();
    refresh
        .iterations()
        .iter()
        .map(|iteration| {
            let number = iteration.id().saved_number().unwrap_or(0);
            let members: Vec<&crate::product_management::story::Story> = all
                .iter()
                .filter(|m| m.iteration_membership().map(|(it, _)| it) == Some(number))
                .collect();
            let position = iteration.builds_outcome().and_then(|outcome_number| {
                outcomes
                    .iter()
                    .find(|o| o.id().saved_number() == Some(outcome_number))
                    .and_then(|o| o.position_of(number))
                    .map(|index| index + 1)
            });
            serde_json::json!({
                "number": number,
                "title": iteration.content.title,
                "iteration_order": iteration.iteration_order,
                "is_current": iteration.is_current,
                "complete": iteration.complete,
                "builds_outcome": iteration.builds_outcome(),
                "outcome_position": position,
                "readiness": iteration_readiness(iteration, &members, workflow),
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
    let all = refresh.all_stories();
    let all_iterations = refresh.iterations();
    let workflow = crate::workflow::current_workflow();
    refresh
        .outcomes()
        .iter()
        .filter(|outcome| !outcome.is_retired())
        .map(|outcome| {
            let number = outcome.id().saved_number().unwrap_or(0);
            let iterations: Vec<serde_json::Value> = building_iterations(outcome, &all_iterations)
                .iter()
                .enumerate()
                .map(|(index, iteration)| {
                    let iteration_number = iteration.id().saved_number().unwrap_or(0);
                    let members: Vec<&crate::product_management::story::Story> = all
                        .iter()
                        .filter(|m| {
                            m.iteration_membership().map(|(it, _)| it) == Some(iteration_number)
                        })
                        .collect();
                    serde_json::json!({
                        "number": iteration_number,
                        "name": iteration.content.title,
                        "position": index as u32 + 1,
                        "readiness": iteration_readiness(iteration, &members, workflow),
                    })
                })
                .collect();
            serde_json::json!({
                "number": number,
                "title": outcome.content.title,
                "body": outcome.content.body.clone().unwrap_or_default(),
                "horizon": outcome.horizon().map(|horizon| horizon.as_str()),
                "horizon_order": outcome.horizon_order(),
                "outcome_readiness": outcome_readiness(outcome, &all_iterations, &all, workflow),
                "iterations": iterations,
            })
        })
        .collect()
}
