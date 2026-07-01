use chrono::{DateTime, Utc};

use super::story_decode::extract_label_names;
use crate::product_management::board::adapters::relationships::{
    OutcomeRelationships, Relationships,
};
use crate::product_management::outcome::{Outcome, OutcomeId, OutcomeLifecycle};
use crate::product_management::story::{BoardState, Story};

pub fn is_outcome_issue(v: &serde_json::Value) -> bool {
    extract_label_names(v)
        .iter()
        .any(|l| l.eq_ignore_ascii_case("outcome"))
}

pub fn decode_outcome(v: &serde_json::Value) -> Option<Outcome> {
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
    let retired = labels.iter().any(|l| l.eq_ignore_ascii_case("retired"));
    Some(assemble_outcome(
        OutcomeId::new(number),
        title,
        body,
        filed_at,
        filed_by,
        &labels,
        retired,
    ))
}

pub fn decode_outcome_from_story(story: &Story) -> Outcome {
    let retired = story.state() == BoardState::Retired;
    assemble_outcome(
        OutcomeId::new(story.id.value()),
        story.title().to_string(),
        story.body().map(str::to_string),
        story.filed_at(),
        story.filed_by().to_string(),
        story.raw_labels(),
        retired,
    )
}

fn assemble_outcome(
    id: OutcomeId,
    title: String,
    body: Option<String>,
    filed_at: DateTime<Utc>,
    filed_by: String,
    labels: &[String],
    retired: bool,
) -> Outcome {
    let lifecycle = if retired {
        OutcomeLifecycle::Retired
    } else {
        OutcomeLifecycle::Active {
            horizon: super::horizon_from_labels(labels),
            horizon_order: super::horizon_order_from_labels(labels),
        }
    };
    let iterations = body
        .as_deref()
        .and_then(OutcomeRelationships::read_from_body)
        .unwrap_or_default()
        .iterations;
    Outcome {
        id,
        title,
        body,
        filed_at,
        filed_by,
        lifecycle,
        iterations,
    }
}
