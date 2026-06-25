use std::sync::Arc;

pub(crate) fn ordered_board_stories(
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    let all = refresh.all_stories();
    let codec = crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
    let workflow = crate::workflow::current_workflow();
    let stories: Vec<serde_json::Value> = all
        .iter()
        .map(|s| {
            let mut json =
                crate::product_management::board::adapters::json::story_to_board_json(&codec, s);
            if crate::product_management::board::adapters::github::story_type_from_labels_or_title(&s.raw_labels, &s.title) == crate::product_management::story::StoryType::Iteration
                && let serde_json::Value::Object(map) = &mut json
            {
                let members: Vec<&crate::product_management::story::Story> = all
                    .iter()
                    .filter(|m| {
                        crate::product_management::board::adapters::github::iteration_membership_from_labels(
                            &m.raw_labels,
                        )
                        .map(|(it, _)| it)
                            == Some(s.number)
                    })
                    .collect();
                let readiness =
                    crate::product_management::iteration::readiness::iteration_readiness(
                        s, &members, workflow,
                    );
                if let Ok(value) = serde_json::to_value(readiness) {
                    map.insert("readiness".to_string(), value);
                }
            }
            if crate::product_management::board::adapters::github::story_type_from_labels_or_title(&s.raw_labels, &s.title) == crate::product_management::story::StoryType::Outcome
                && let serde_json::Value::Object(map) = &mut json
            {
                let rolled = crate::product_management::iteration::readiness::outcome_readiness(
                    s, &all, workflow,
                );
                if let Ok(value) = serde_json::to_value(rolled) {
                    map.insert("outcome_readiness".to_string(), value);
                }
            }
            json
        })
        .collect();
    crate::product_management::board::ready_order_label::sort_board_ready_by_membership(stories)
}
