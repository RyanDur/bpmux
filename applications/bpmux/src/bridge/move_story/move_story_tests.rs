use super::*;
use crate::product_management::story::Stage;

#[test]
fn target_state_parses_canonical_spellings() {
    assert_eq!(
        TargetState::parse("ready").unwrap(),
        TargetState::Workflow(BoardState::Ready)
    );
    assert_eq!(
        TargetState::parse("In-Progress").unwrap(),
        TargetState::Workflow(BoardState::InProgress(Stage::Dev))
    );
    assert_eq!(TargetState::parse("retired").unwrap(), TargetState::Retired);
    assert_eq!(
        TargetState::parse("Accepted").unwrap(),
        TargetState::Workflow(BoardState::Accepted)
    );
}

#[test]
fn target_state_parse_unknown_lists_valid_options() {
    let err = TargetState::parse("bogus").unwrap_err();
    assert!(
        err.contains("retired") && err.contains("ready") && err.contains("delivered"),
        "error must enumerate valid options for the agent: got {err}"
    );
}

#[test]
fn target_state_rejects_noncanonical_spellings() {
    assert!(TargetState::parse("in progress").is_err());
    assert!(TargetState::parse("deskcheck").is_err());
}

#[test]
fn closes_issue_is_true_only_for_terminal_states() {
    assert!(TargetState::Retired.closes_issue());
    assert!(TargetState::Workflow(BoardState::Accepted).closes_issue());
    assert!(!TargetState::Workflow(BoardState::Acceptance).closes_issue());
    assert!(!TargetState::Workflow(BoardState::Ready).closes_issue());
    assert!(!TargetState::Workflow(BoardState::Icebox).closes_issue());
}

#[test]
fn parse_position_rejects_zero_and_non_integer() {
    let mut args: Map<String, Value> = Map::new();
    args.insert("position".to_string(), serde_json::json!(0));
    assert!(parse_position(&args).is_err());
    args.insert("position".to_string(), serde_json::json!("first"));
    assert!(parse_position(&args).is_err());
}

#[test]
fn parse_position_returns_none_when_absent_or_null() {
    let mut args: Map<String, Value> = Map::new();
    assert_eq!(parse_position(&args).unwrap(), None);
    args.insert("position".to_string(), Value::Null);
    assert_eq!(parse_position(&args).unwrap(), None);
}

#[test]
fn parse_position_accepts_one_based_positive_integer() {
    let mut args: Map<String, Value> = Map::new();
    args.insert("position".to_string(), serde_json::json!(1));
    assert_eq!(parse_position(&args).unwrap(), Some(1));
    args.insert("position".to_string(), serde_json::json!(42));
    assert_eq!(parse_position(&args).unwrap(), Some(42));
}

#[test]
fn require_nonempty_rejects_blank_or_whitespace_only() {
    let mut args: Map<String, Value> = Map::new();
    args.insert("rationale".to_string(), Value::String("".to_string()));
    assert!(require_nonempty(&args, "rationale", "hint").is_err());
    args.insert(
        "rationale".to_string(),
        Value::String("   \t\n".to_string()),
    );
    assert!(require_nonempty(&args, "rationale", "hint").is_err());
}

#[test]
fn require_nonempty_trims_returned_value() {
    let mut args: Map<String, Value> = Map::new();
    args.insert(
        "rationale".to_string(),
        Value::String("  cleanup before iteration close  ".to_string()),
    );
    let v = require_nonempty(&args, "rationale", "hint").unwrap();
    assert_eq!(v, "cleanup before iteration close");
}

#[test]
fn audit_comment_body_includes_actor_transition_position_rationale() {
    let body = audit_comment_body(
        "dev",
        "delivered",
        TargetState::Workflow(BoardState::Ready),
        Some(3),
        "rework after desk-check",
    );
    assert!(body.contains("**move_story** by dev"));
    assert!(body.contains("delivered → ready"));
    assert!(body.contains("position=3"));
    assert!(body.contains("rework after desk-check"));
}

#[test]
fn audit_comment_body_omits_position_segment_when_none() {
    let body = audit_comment_body(
        "pm",
        "icebox",
        TargetState::Workflow(BoardState::Ready),
        None,
        "next-iteration promote",
    );
    assert!(body.contains("icebox → ready"));
    assert!(
        !body.contains("position="),
        "absent position must not surface a position= segment in the audit body"
    );
}
