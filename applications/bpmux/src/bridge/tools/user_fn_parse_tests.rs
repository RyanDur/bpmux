#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::user_fn_parse::{parse_definition, substitute_args};

#[test]
fn parses_minimal_frontmatter_and_body() {
    let text = "---\nname: walk_state_summary\ndescription: Walk the team state.\ntype: user_fn\n---\nBody here with {{topic}}.\n";
    let def = parse_definition(text).unwrap();
    assert_eq!(def.name, "walk_state_summary");
    assert_eq!(def.description, "Walk the team state.");
    assert!(def.args.is_empty());
    assert_eq!(def.body, "Body here with {{topic}}.\n");
}

#[test]
fn parses_args_list() {
    let text = "---\nname: greet\ndescription: Greet someone\ntype: user_fn\nargs: [name, mood]\n---\nHi {{name}}, you seem {{mood}}.\n";
    let def = parse_definition(text).unwrap();
    assert_eq!(def.args, vec!["name".to_string(), "mood".to_string()]);
}

#[test]
fn rejects_missing_name() {
    let text = "---\ndescription: x\ntype: user_fn\n---\nbody";
    assert!(parse_definition(text).unwrap_err().contains("name"));
}

#[test]
fn rejects_wrong_type() {
    let text = "---\nname: x\ndescription: x\ntype: lesson\n---\nbody";
    assert!(parse_definition(text).unwrap_err().contains("user_fn"));
}

#[test]
fn rejects_missing_frontmatter() {
    let text = "no frontmatter here";
    assert!(parse_definition(text).unwrap_err().contains("fence"));
}

#[test]
fn substitutes_known_args() {
    let body = "Hello {{name}}, ratio is {{ratio}}.";
    let mut args = serde_json::Map::new();
    args.insert("name".into(), serde_json::json!("dev"));
    args.insert("ratio".into(), serde_json::json!(4.5));
    let out = substitute_args(body, &args).unwrap();
    assert_eq!(out, "Hello dev, ratio is 4.5.");
}

#[test]
fn substitution_returns_err_with_missing_key() {
    let body = "Hello {{name}}, you forgot {{topic}}.";
    let mut args = serde_json::Map::new();
    args.insert("name".into(), serde_json::json!("dev"));
    let err = substitute_args(body, &args).unwrap_err();
    assert_eq!(err, "topic");
}

#[test]
fn substitution_handles_repeated_slots() {
    let body = "{{x}} and {{x}} again";
    let mut args = serde_json::Map::new();
    args.insert("x".into(), serde_json::json!("ok"));
    assert_eq!(substitute_args(body, &args).unwrap(), "ok and ok again");
}

#[test]
fn substitution_no_slots_passes_through() {
    let body = "no slots here";
    let args = serde_json::Map::new();
    assert_eq!(substitute_args(body, &args).unwrap(), body);
}
