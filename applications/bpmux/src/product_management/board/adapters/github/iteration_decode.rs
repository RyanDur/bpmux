use chrono::{DateTime, Utc};

use super::label_fields::{
    is_current_from_labels, iteration_dev_ok_by_from_labels, iteration_order_from_labels,
    iteration_review_closers_from_labels,
};
use super::story_decode::extract_label_names;
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, Relationships,
};
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::story::Story;

pub fn is_iteration_issue(v: &serde_json::Value) -> bool {
    extract_label_names(v)
        .iter()
        .any(|l| l.eq_ignore_ascii_case("iteration"))
}

pub fn decode_iteration(v: &serde_json::Value) -> Option<Iteration> {
    let number = v.get("number").and_then(|x| x.as_u64())?;
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let labels = extract_label_names(v);
    let body = v
        .get("body")
        .and_then(|x| x.as_str())
        .map(ToString::to_string);
    let filed_at = v
        .get("created_at")
        .and_then(|x| x.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let filed_by = v
        .get("user")
        .and_then(|u| u.get("login"))
        .and_then(|x| x.as_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".to_string());
    let complete = match v.get("gh_state").and_then(|x| x.as_str()) {
        Some(s) => matches!(s, "closed" | "CLOSED"),
        None => matches!(
            v.get("state").and_then(|x| x.as_str()),
            Some("closed" | "CLOSED")
        ),
    };
    Some(assemble_iteration(
        IterationId::new(number),
        title,
        body,
        filed_at,
        filed_by,
        &labels,
        complete,
    ))
}

pub fn decode_iteration_from_story(story: &Story) -> Iteration {
    assemble_iteration(
        IterationId::new(story.id.value()),
        story.title().to_string(),
        story.body().map(str::to_string),
        story.filed_at(),
        story.filed_by().to_string(),
        story.raw_labels(),
        story.is_closed,
    )
}

fn assemble_iteration(
    id: IterationId,
    title: String,
    body: Option<String>,
    filed_at: DateTime<Utc>,
    filed_by: String,
    labels: &[String],
    complete: bool,
) -> Iteration {
    let relationships = body
        .as_deref()
        .and_then(IterationRelationships::read_from_body)
        .unwrap_or_default();
    Iteration {
        id,
        title,
        body,
        filed_at,
        filed_by,
        outcome: relationships.outcome,
        iteration_order: iteration_order_from_labels(labels),
        stories: relationships.stories,
        review_closers: iteration_review_closers_from_labels(labels),
        dev_ok_by: iteration_dev_ok_by_from_labels(labels),
        is_current: is_current_from_labels(labels),
        complete,
    }
}
