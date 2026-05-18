#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;

use tempfile::TempDir;

use super::bundled_default::bundled_default;
use super::loader::{WorkflowLoadError, load_workflow, workflow_path};

fn write_workflow(tmp: &TempDir, contents: &str) {
    let path = workflow_path(tmp.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, contents).unwrap();
}

#[test]
fn missing_workflow_file_returns_bundled_default() {
    let tmp = TempDir::new().unwrap();
    let result = load_workflow(tmp.path()).expect("missing should yield default");
    assert_eq!(result, bundled_default());
}

#[test]
fn valid_workflow_file_parses_to_config() {
    let tmp = TempDir::new().unwrap();
    write_workflow(
        &tmp,
        r#"
[[section]]
id = "pre-work"

[[section]]
id = "in-work"

[[section]]
id = "post-work"

[[state]]
id = "icebox"
section = "pre-work"

[[state]]
id = "ready"
section = "pre-work"
"#,
    );
    let result = load_workflow(tmp.path()).expect("valid TOML should parse");
    assert_eq!(result.section.len(), 3);
    assert_eq!(result.section[0].id, "pre-work");
    assert_eq!(result.state.len(), 2);
    assert_eq!(result.state[0].id, "icebox");
    assert_eq!(result.state[0].section, "pre-work");
}

#[test]
fn malformed_toml_hard_fails_with_structured_error_never_silent_fallback() {
    let tmp = TempDir::new().unwrap();
    write_workflow(&tmp, "this is not [[ valid toml = =");
    let err = load_workflow(tmp.path()).expect_err("malformed should error");
    match err {
        WorkflowLoadError::ParseFailed { path, parse_error } => {
            assert!(
                path.ends_with(".bpmux/workflow.toml"),
                "path cites canonical location: {path}"
            );
            assert!(
                !parse_error.is_empty(),
                "parse_error names the TOML failure"
            );
        }
        other => panic!("expected ParseFailed, got {other:?}"),
    }
}

#[test]
fn missing_required_field_in_state_hard_fails() {
    let tmp = TempDir::new().unwrap();
    write_workflow(
        &tmp,
        r#"
[[state]]
id = "icebox"
# section field missing — should hard-fail
"#,
    );
    let err = load_workflow(tmp.path()).expect_err("missing required field should error");
    assert!(matches!(err, WorkflowLoadError::ParseFailed { .. }));
}

#[test]
fn workflow_toml_overrides_evidence_window_seconds_per_transition() {
    let tmp = TempDir::new().unwrap();
    write_workflow(
        &tmp,
        r#"
[[section]]
id = "pre-work"

[[section]]
id = "in-work"

[[section]]
id = "post-work"

[[state]]
id = "delivered"
section = "in-work"

[[state]]
id = "accepted"
section = "post-work"

[[transition]]
id = "accept"
from = "delivered"
to = "accepted"
actor_roles = ["pm"]
gate_class = "team-verifiable"
evidence_window_seconds = 120
required_evidence_roles = ["designer", "dev"]
"#,
    );
    let workflow = load_workflow(tmp.path()).expect("valid TOML should parse");
    let t = workflow
        .transition
        .iter()
        .find(|t| t.id == "accept")
        .expect("accept transition present");
    assert_eq!(t.evidence_window_seconds, 120);
    assert_eq!(t.required_evidence_roles, vec!["designer", "dev"]);
}

#[test]
fn workflow_toml_supports_per_class_accept_transitions() {
    let tmp = TempDir::new().unwrap();
    write_workflow(
        &tmp,
        r#"
[[transition]]
id = "accept"
from = "delivered"
to = "accepted"
actor_roles = ["ryan", "pm"]
gate_class = "team-verifiable"
evidence_window_seconds = 600
required_evidence_roles = ["designer", "dev"]

[[transition]]
id = "accept"
from = "delivered"
to = "accepted"
actor_roles = ["ryan"]
gate_class = "owner-walk"
evidence_window_seconds = 0
required_evidence_roles = []
"#,
    );
    let workflow = load_workflow(tmp.path()).expect("valid TOML should parse");
    assert_eq!(
        workflow
            .transition
            .iter()
            .filter(|t| t.id == "accept")
            .count(),
        2,
        "both class variants parse"
    );
    use super::config::GateClass;
    let team_verifiable = workflow
        .transition_for("accept", GateClass::TeamVerifiable)
        .expect("team-verifiable variant");
    assert_eq!(team_verifiable.actor_roles, vec!["ryan", "pm"]);
    let owner_walk = workflow
        .transition_for("accept", GateClass::OwnerWalk)
        .expect("owner-walk variant");
    assert_eq!(owner_walk.actor_roles, vec!["ryan"]);
}

#[test]
fn structured_error_serializes_with_kind_discriminator() {
    let err = WorkflowLoadError::ParseFailed {
        path: "/tmp/x/.bpmux/workflow.toml".into(),
        parse_error: "expected `=` after key".into(),
    };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "workflow-config-invalid");
    assert_eq!(json["path"], "/tmp/x/.bpmux/workflow.toml");
    assert_eq!(json["parse_error"], "expected `=` after key");
}
