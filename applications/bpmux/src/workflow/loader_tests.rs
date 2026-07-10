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

[[state]]
id = "in-progress"
section = "in-work"

[[state]]
id = "accepted"
section = "post-work"
"#,
    );
    let result = load_workflow(tmp.path()).expect("valid TOML should parse");
    assert_eq!(result.section.len(), 3);
    assert_eq!(result.section[0].id, "pre-work");
    assert_eq!(result.state.len(), 4);
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
fn workflow_toml_with_non_canonical_section_hard_fails() {
    let tmp = TempDir::new().unwrap();
    write_workflow(
        &tmp,
        r#"
[[state]]
id = "staging"
section = "deployment"
"#,
    );
    let err = load_workflow(tmp.path()).expect_err("non-canonical section should fail");
    match err {
        WorkflowLoadError::CanonViolation {
            invalid_section,
            valid_sections,
            state_id,
        } => {
            assert_eq!(invalid_section, "deployment");
            assert_eq!(state_id, "staging");
            assert!(valid_sections.contains(&"pre-work".to_string()));
            assert!(valid_sections.contains(&"in-work".to_string()));
            assert!(valid_sections.contains(&"post-work".to_string()));
        }
        other => panic!("expected CanonViolation, got {other:?}"),
    }
}

#[test]
fn workflow_toml_with_empty_states_hard_fails_per_canonical_section() {
    let tmp = TempDir::new().unwrap();
    write_workflow(&tmp, "");
    let err = load_workflow(tmp.path()).expect_err("empty workflow.toml should fail");
    match err {
        WorkflowLoadError::EmptyStates { missing_sections } => {
            assert_eq!(missing_sections.len(), 3);
            assert!(missing_sections.contains(&"pre-work".to_string()));
            assert!(missing_sections.contains(&"in-work".to_string()));
            assert!(missing_sections.contains(&"post-work".to_string()));
        }
        other => panic!("expected EmptyStates, got {other:?}"),
    }
}

#[test]
fn workflow_toml_with_state_in_some_but_not_all_canonical_sections_hard_fails() {
    let tmp = TempDir::new().unwrap();
    write_workflow(
        &tmp,
        r#"
[[state]]
id = "icebox"
section = "pre-work"

[[state]]
id = "ready"
section = "pre-work"
"#,
    );
    let err = load_workflow(tmp.path()).expect_err("partial-coverage workflow.toml should fail");
    match err {
        WorkflowLoadError::EmptyStates { missing_sections } => {
            assert!(missing_sections.contains(&"in-work".to_string()));
            assert!(missing_sections.contains(&"post-work".to_string()));
        }
        other => panic!("expected EmptyStates, got {other:?}"),
    }
}

#[test]
fn canon_violation_json_shape() {
    let err = WorkflowLoadError::CanonViolation {
        invalid_section: "deployment".into(),
        valid_sections: vec!["pre-work".into(), "in-work".into(), "post-work".into()],
        state_id: "staging".into(),
    };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "workflow-canon-violation");
    assert_eq!(json["invalid_section"], "deployment");
    assert_eq!(json["state_id"], "staging");
}

#[test]
fn workflow_toml_with_multiple_subsections_per_canonical_section_parses_cleanly() {
    let tmp = TempDir::new().unwrap();
    write_workflow(
        &tmp,
        r#"
[[state]]
id = "ideas"
section = "pre-work"

[[state]]
id = "triaged"
section = "pre-work"

[[state]]
id = "vetted"
section = "pre-work"

[[state]]
id = "refined"
section = "pre-work"

[[state]]
id = "ready"
section = "pre-work"

[[state]]
id = "in-progress"
section = "in-work"

[[state]]
id = "accepted"
section = "post-work"
"#,
    );
    let workflow = load_workflow(tmp.path()).expect("multi-subsection workflow.toml parses");
    assert_eq!(
        workflow
            .state
            .iter()
            .filter(|s| s.section == "pre-work")
            .count(),
        5
    );
    assert_eq!(
        workflow
            .state
            .iter()
            .filter(|s| s.section == "in-work")
            .count(),
        1
    );
    assert_eq!(
        workflow
            .state
            .iter()
            .filter(|s| s.section == "post-work")
            .count(),
        1
    );
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
id = "ready"
section = "pre-work"

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
[[state]]
id = "ready"
section = "pre-work"

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
actor_roles = ["owner", "pm"]
gate_class = "team-verifiable"
evidence_window_seconds = 600
required_evidence_roles = ["designer", "dev"]

[[transition]]
id = "accept"
from = "delivered"
to = "accepted"
actor_roles = ["owner"]
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
    assert_eq!(team_verifiable.actor_roles, vec!["owner", "pm"]);
    let owner_walk = workflow
        .transition_for("accept", GateClass::OwnerWalk)
        .expect("owner-walk variant");
    assert_eq!(owner_walk.actor_roles, vec!["owner"]);
}

#[test]
fn state_judges_and_fail_to_parse_and_default_open() {
    let tmp = TempDir::new().unwrap();
    write_workflow(
        &tmp,
        r#"
[[state]]
id = "ready"
section = "pre-work"

[[state]]
id = "build"
section = "in-work"
judges = ["dev"]

[[state]]
id = "review"
section = "in-work"
judges = ["designer", "qa:claude-code"]
fail_to = "build"

[[state]]
id = "accepted"
section = "post-work"
"#,
    );
    let workflow = load_workflow(tmp.path()).expect("judges/fail_to TOML parses");
    assert_eq!(workflow.judges_of("build"), &["dev".to_string()]);
    assert_eq!(
        workflow.judges_of("review"),
        &["designer".to_string(), "qa:claude-code".to_string()]
    );
    assert_eq!(workflow.fail_to_of("review"), Some("build"));
    assert!(
        workflow.judges_of("ready").is_empty(),
        "no judges configured means anyone may judge"
    );
    assert_eq!(
        workflow.fail_to_of("build"),
        None,
        "no fail_to configured means the default ladder applies"
    );
}

#[test]
fn explicit_config_file_override_beats_the_project_root_file() {
    use super::loader::load_workflow_with_override;
    let root = TempDir::new().unwrap();
    write_workflow(
        &root,
        r#"
[[state]]
id = "ready"
section = "pre-work"

[[state]]
id = "in-progress"
section = "in-work"

[[state]]
id = "accepted"
section = "post-work"
"#,
    );
    let override_file = root.path().join("elsewhere.toml");
    fs::write(
        &override_file,
        r#"
[[state]]
id = "triage"
section = "pre-work"

[[state]]
id = "build"
section = "in-work"

[[state]]
id = "done"
section = "post-work"
"#,
    )
    .unwrap();
    let workflow = load_workflow_with_override(Some(&override_file), root.path())
        .expect("override file loads");
    assert_eq!(
        workflow.state[0].id, "triage",
        "the explicit override wins over the project-root .bpmux/workflow.toml"
    );
}

#[test]
fn explicit_config_file_override_missing_is_a_loud_error_never_a_fallback() {
    use super::loader::load_workflow_with_override;
    let root = TempDir::new().unwrap();
    let err = load_workflow_with_override(Some(&root.path().join("missing.toml")), root.path())
        .expect_err("an explicitly named config that cannot be read must never fall back");
    assert!(matches!(err, WorkflowLoadError::ReadFailed { .. }));
}

#[test]
fn bundled_default_is_the_three_state_single_middle_workflow() {
    let workflow = bundled_default();
    let middle: Vec<&str> = workflow
        .state
        .iter()
        .filter(|s| s.section == "in-work")
        .map(|s| s.id.as_str())
        .collect();
    assert_eq!(
        middle,
        vec!["in-progress"],
        "no config file means the default three states: ready, one undivided in-progress, acceptance"
    );
    assert!(
        workflow.state.iter().all(|s| s.judges.is_empty()),
        "the bundled default configures no judges — anyone may judge"
    );
    assert!(
        workflow.transition_by_id("pass-desk-check").is_none(),
        "desk-check is the team config's stage, not the default's"
    );
    let deliver = workflow
        .transition_by_id("deliver")
        .expect("the single middle still exits via deliver");
    assert_eq!(deliver.to, "acceptance");
}

#[test]
fn the_repo_workflow_toml_is_the_team_two_stage_config() {
    let workflow = crate::workflow::test_support::repo_workflow();
    let middle: Vec<&str> = workflow
        .state
        .iter()
        .filter(|s| s.section == "in-work")
        .map(|s| s.id.as_str())
        .collect();
    assert_eq!(middle, vec!["in-progress", "desk-check"]);
    assert_eq!(workflow.judges_of("in-progress"), &["dev".to_string()]);
    assert_eq!(workflow.judges_of("desk-check"), &["designer".to_string()]);
    assert!(
        workflow.transition_by_id("pass-desk-check").is_some(),
        "the team ceremony verbs stay configured"
    );
}

#[test]
fn legacy_role_id_key_still_loads_via_alias() {
    let tmp = TempDir::new().unwrap();
    write_workflow(
        &tmp,
        r#"
[[role]]
id = "dev"
purposes = ["dev"]

[[state]]
id = "ready"
section = "pre-work"

[[state]]
id = "in-progress"
section = "in-work"

[[state]]
id = "accepted"
section = "post-work"
"#,
    );
    let workflow = load_workflow(tmp.path())
        .expect("a workflow.toml written before the rename (role id key) still loads");
    assert_eq!(workflow.role_name_for_purpose("dev"), Some("dev"));
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
