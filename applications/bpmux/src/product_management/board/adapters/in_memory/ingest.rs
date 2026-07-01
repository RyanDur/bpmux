use std::collections::HashMap;

use crate::product_management::board::adapters::github::{
    decode_iteration_from_story, decode_outcome_from_story,
};
use crate::product_management::board::item::ProductItem;
use crate::product_management::iteration::Iteration;
use crate::product_management::outcome::Outcome;
use crate::product_management::story::Story;

pub(super) fn classify_ingested(
    stories: impl Iterator<Item = Story>,
    labels: &HashMap<u64, Vec<String>>,
) -> (Vec<ProductItem>, Vec<Outcome>, Vec<Iteration>) {
    let mut items = Vec::new();
    let mut outcomes = Vec::new();
    let mut iterations = Vec::new();
    for mut story in stories {
        if let Some(labels) = labels.get(&story.id.value()) {
            story.raw_labels = labels.clone();
        }
        let has_label = |needle: &str| {
            story
                .raw_labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case(needle))
        };
        if has_label("outcome") {
            outcomes.push(decode_outcome_from_story(&story));
        } else if has_label("iteration") {
            iterations.push(decode_iteration_from_story(&story));
        } else {
            items.push(ProductItem::from_story(&story));
        }
    }
    (items, outcomes, iterations)
}
