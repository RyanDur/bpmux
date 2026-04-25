//! #419 rework-context enrichment.
//!
//! When an agent fetches a rework-labeled story (rejected / needs
//! fixes), the top of the response should name WHY it's being
//! reworked — not force comment-archaeology at every pickup + desk-
//! check + re-audit. This module turns the comment trail into a
//! compact context block that prepends the story body.
//!
//! Shape is pure: takes labels + comments (both as JSON values
//! matching the tracker's existing serialization), returns the enriched
//! body string (or None if the story isn't in a rework state).

const REWORK_LABELS: &[&str] = &["rejected", "needs fixes"];
const REJECTION_MARKER: &str = "**rejection_reason**";
const VERIFIED_OBSERVATION_MARKER: &str = "**verified_observation**";
const CONTEXT_HEADER: &str = "## Rework context";
const CONTEXT_SEPARATOR: &str = "---";

pub fn is_rework_labeled(labels: &[String]) -> bool {
    labels
        .iter()
        .any(|l| REWORK_LABELS.iter().any(|rl| l.eq_ignore_ascii_case(rl)))
}

pub fn labels_from_story_json(story: &serde_json::Value) -> Vec<String> {
    story
        .get("labels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|l| {
                    l.as_str().map(ToString::to_string).or_else(|| {
                        l.get("name")
                            .and_then(|n| n.as_str())
                            .map(ToString::to_string)
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn latest_comment_body_with_marker<'a>(
    comments: &'a [serde_json::Value],
    marker: &str,
) -> Option<&'a str> {
    comments.iter().rev().find_map(|c| {
        let body = c.get("body").and_then(|v| v.as_str())?;
        if body.contains(marker) {
            Some(body)
        } else {
            None
        }
    })
}

pub fn build_rework_context(comments: &[serde_json::Value]) -> Option<String> {
    let rejection = latest_comment_body_with_marker(comments, REJECTION_MARKER);
    let prior_delivery = latest_comment_body_with_marker(comments, VERIFIED_OBSERVATION_MARKER);

    if rejection.is_none() && prior_delivery.is_none() {
        return None;
    }

    let mut out = String::from(CONTEXT_HEADER);
    out.push_str("\n\n");

    if let Some(r) = rejection {
        out.push_str(r);
        out.push_str("\n\n");
    }

    if let Some(p) = prior_delivery {
        out.push_str(p);
        out.push_str("\n\n");
    }

    out.push_str(CONTEXT_SEPARATOR);
    out.push('\n');
    Some(out)
}

pub fn enrich_body(original_body: &str, context: &str) -> String {
    format!("{context}\n{original_body}")
}
