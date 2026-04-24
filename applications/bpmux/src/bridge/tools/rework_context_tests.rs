#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::rework_context::{
    build_rework_context, enrich_body, is_rework_labeled, labels_from_story_json,
};

#[test]
fn rework_labeled_true_for_rejected() {
    assert!(is_rework_labeled(&["rejected".to_string()]));
}

#[test]
fn rework_labeled_true_for_needs_fixes() {
    assert!(is_rework_labeled(&[
        "ready".to_string(),
        "needs fixes".to_string()
    ]));
}

#[test]
fn rework_labeled_false_for_plain_ready() {
    assert!(!is_rework_labeled(&["ready".to_string()]));
}

#[test]
fn rework_labeled_case_insensitive() {
    assert!(is_rework_labeled(&["Rejected".to_string()]));
}

#[test]
fn labels_from_story_accepts_string_array() {
    let story = serde_json::json!({"labels": ["ready", "rejected"]});
    assert_eq!(labels_from_story_json(&story), vec!["ready", "rejected"]);
}

#[test]
fn labels_from_story_accepts_github_object_shape() {
    let story = serde_json::json!({"labels": [{"name": "ready"}, {"name": "rejected"}]});
    assert_eq!(labels_from_story_json(&story), vec!["ready", "rejected"]);
}

#[test]
fn labels_from_story_empty_when_missing() {
    let story = serde_json::json!({});
    assert_eq!(labels_from_story_json(&story), Vec::<String>::new());
}

#[test]
fn build_rework_context_none_when_no_markers() {
    let comments = vec![serde_json::json!({"body": "some unrelated comment"})];
    assert!(build_rework_context(&comments).is_none());
}

#[test]
fn build_rework_context_picks_latest_rejection() {
    let comments = vec![
        serde_json::json!({"body": "**rejection_reason** (reject-gate):\n\nearly reject", "created_at": "2026-04-20T00:00:00Z"}),
        serde_json::json!({"body": "**rejection_reason** (reject-gate):\n\nlatest reject", "created_at": "2026-04-24T00:00:00Z"}),
    ];
    let ctx = build_rework_context(&comments).expect("should build context");
    assert!(
        ctx.contains("latest reject"),
        "should surface latest: {ctx}"
    );
    assert!(!ctx.contains("early reject"), "should drop earlier: {ctx}");
}

#[test]
fn build_rework_context_includes_verified_observation() {
    let comments = vec![
        serde_json::json!({"body": "**verified_observation** (dev deliver-gate):\n\nran the feature end-to-end"}),
        serde_json::json!({"body": "**rejection_reason** (reject-gate):\n\nobservation didn't match user-story"}),
    ];
    let ctx = build_rework_context(&comments).expect("should build context");
    assert!(ctx.contains("verified_observation"));
    assert!(ctx.contains("ran the feature end-to-end"));
    assert!(ctx.contains("rejection_reason"));
    assert!(ctx.contains("observation didn't match user-story"));
}

#[test]
fn build_rework_context_starts_with_section_header() {
    let comments = vec![serde_json::json!({"body": "**rejection_reason** (reject-gate):\n\ntext"})];
    let ctx = build_rework_context(&comments).unwrap();
    assert!(
        ctx.starts_with("## Rework context"),
        "header missing: {ctx}"
    );
}

#[test]
fn build_rework_context_ends_with_separator() {
    let comments = vec![serde_json::json!({"body": "**rejection_reason** (reject-gate):\n\ntext"})];
    let ctx = build_rework_context(&comments).unwrap();
    assert!(ctx.trim_end().ends_with("---"), "separator missing: {ctx}");
}

#[test]
fn enrich_body_prepends_context() {
    let ctx = "## Rework context\n\n**rejection_reason** ...\n\n---\n";
    let body = "original story body";
    let enriched = enrich_body(body, ctx);
    assert!(enriched.starts_with("## Rework context"));
    assert!(enriched.ends_with("original story body"));
}

#[test]
fn build_rework_context_only_verified_observation_still_builds() {
    let comments =
        vec![serde_json::json!({"body": "**verified_observation** (dev deliver-gate):\n\nwork"})];
    let ctx = build_rework_context(&comments).expect("should build even without rejection");
    assert!(ctx.contains("verified_observation"));
}
