use super::label_fields::{
    is_current_from_labels, iteration_dev_ok_by_from_labels, iteration_order_from_labels,
    iteration_review_closers_from_labels,
};
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, Relationships,
};
use crate::product_management::board::item::Content;
use crate::product_management::iteration::Iteration;
use crate::product_management::story::Story;

pub fn decode_iteration(story: &Story) -> Iteration {
    let labels = story.raw_labels();
    let relationships = story
        .body()
        .and_then(IterationRelationships::read_from_body)
        .unwrap_or_default();
    Iteration {
        content: Content {
            id: story.key(),
            title: story.title().to_string(),
            body: story.body().map(str::to_string),
            filed_at: story.filed_at(),
            filed_by: story.filed_by().to_string(),
        },
        outcome: relationships.outcome,
        iteration_order: iteration_order_from_labels(labels),
        stories: relationships.stories,
        review_closers: iteration_review_closers_from_labels(labels),
        dev_ok_by: iteration_dev_ok_by_from_labels(labels),
        is_current: is_current_from_labels(labels),
        complete: story.is_closed,
    }
}
