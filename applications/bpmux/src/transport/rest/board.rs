use std::sync::Arc;

pub(crate) fn ordered_board_stories(
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    let all = refresh.all_stories();
    let codec = crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
    let workflow = crate::workflow::current_workflow();
    let items: Vec<serde_json::Value> = all
        .iter()
        .map(|s| {
            let mut json =
                crate::product_management::board::adapters::json::story_to_board_json(&codec, s);
            if s.story_type() == crate::product_management::story::StoryType::Iteration
                && let serde_json::Value::Object(map) = &mut json
            {
                let members: Vec<&crate::product_management::story::Story> = all
                    .iter()
                    .filter(|m| m.iteration_membership().map(|(it, _)| it) == Some(s.number))
                    .collect();
                let iteration = crate::product_management::iteration::Iteration::from_story(s);
                let readiness =
                    crate::product_management::iteration::readiness::iteration_readiness(
                        &iteration, &members, workflow,
                    );
                if let Ok(value) = serde_json::to_value(readiness) {
                    map.insert("readiness".to_string(), value);
                }
            }
            json
        })
        .collect();
    crate::product_management::board::ready_order_label::sort_board_ready_by_membership(items)
}

pub(crate) fn board_outcomes(
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    use crate::product_management::iteration::readiness::{
        building_iterations, iteration_readiness, outcome_readiness,
    };
    let all = refresh.all_stories();
    let workflow = crate::workflow::current_workflow();
    refresh
        .outcomes()
        .iter()
        .filter(|outcome| !outcome.is_retired())
        .map(|outcome| {
            let number = outcome.id().saved_number().unwrap_or(0);
            let iterations: Vec<serde_json::Value> = building_iterations(number, &all)
                .iter()
                .map(|iteration| {
                    let iteration_number = iteration.id().saved_number().unwrap_or(0);
                    let members: Vec<&crate::product_management::story::Story> = all
                        .iter()
                        .filter(|m| {
                            m.iteration_membership().map(|(it, _)| it) == Some(iteration_number)
                        })
                        .collect();
                    let position = iteration
                        .outcome_membership
                        .map(|(_, position)| position)
                        .unwrap_or(0);
                    serde_json::json!({
                        "number": iteration_number,
                        "name": iteration.content.title,
                        "position": position,
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
                "outcome_readiness": outcome_readiness(number, &all, workflow),
                "iterations": iterations,
            })
        })
        .collect()
}
