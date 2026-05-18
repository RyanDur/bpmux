#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::Utc;

use super::bundled_default::bundled_default;
use super::config::{GateClass, Transition, WorkflowConfig};
use super::gate::{EvidenceMessage, GateError, check_evidence_present, check_role_allowed};

#[test]
fn bundled_default_allows_ryan_to_accept() {
    let workflow = bundled_default();
    assert!(check_role_allowed(&workflow, "accept", "ryan").is_ok());
}

#[test]
fn bundled_default_allows_pm_to_accept() {
    let workflow = bundled_default();
    assert!(check_role_allowed(&workflow, "accept", "pm").is_ok());
}

#[test]
fn bundled_default_refuses_designer_to_accept() {
    let workflow = bundled_default();
    let err = check_role_allowed(&workflow, "accept", "designer").expect_err("designer refused");
    match err {
        GateError::RoleNotAllowed {
            actor_role,
            attempted_transition,
            allowed_roles,
        } => {
            assert_eq!(actor_role, "designer");
            assert_eq!(attempted_transition, "accept");
            assert!(allowed_roles.contains(&"ryan".to_string()));
            assert!(allowed_roles.contains(&"pm".to_string()));
            assert!(!allowed_roles.contains(&"designer".to_string()));
        }
        other => panic!("expected RoleNotAllowed, got {other:?}"),
    }
}

#[test]
fn bundled_default_refuses_dev_to_accept() {
    let workflow = bundled_default();
    assert!(matches!(
        check_role_allowed(&workflow, "accept", "dev"),
        Err(GateError::RoleNotAllowed { .. })
    ));
}

#[test]
fn unknown_transition_returns_transition_undefined() {
    let workflow = WorkflowConfig {
        section: vec![],
        state: vec![],
        transition: vec![],
    };
    let err = check_role_allowed(&workflow, "accept", "pm").expect_err("undefined transition");
    assert!(matches!(err, GateError::TransitionUndefined { .. }));
}

#[test]
fn transition_with_only_one_role_refuses_other_roles() {
    let workflow = WorkflowConfig {
        section: vec![],
        state: vec![],
        transition: vec![Transition {
            id: "accept".into(),
            from: "delivered".into(),
            to: "accepted".into(),
            actor_roles: vec!["coordinator".into()],
            gate_class: GateClass::None,
            evidence_window_seconds: 0,
            required_evidence_roles: vec![],
        }],
    };
    assert!(check_role_allowed(&workflow, "accept", "coordinator").is_ok());
    let err = check_role_allowed(&workflow, "accept", "implementer").expect_err("not allowed");
    if let GateError::RoleNotAllowed { allowed_roles, .. } = err {
        assert_eq!(allowed_roles, vec!["coordinator".to_string()]);
    } else {
        panic!("expected RoleNotAllowed");
    }
}

#[test]
fn json_shape_includes_kind_discriminator_and_structured_fields() {
    let err = GateError::RoleNotAllowed {
        actor_role: "dev".into(),
        attempted_transition: "accept".into(),
        allowed_roles: vec!["ryan".into(), "pm".into()],
    };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "workflow-role-not-allowed");
    assert_eq!(json["actor_role"], "dev");
    assert_eq!(json["attempted_transition"], "accept");
    assert_eq!(json["allowed_roles"][0], "ryan");
    assert_eq!(json["allowed_roles"][1], "pm");
}

fn msg(from_role: &str, seconds_ago: i64) -> EvidenceMessage {
    EvidenceMessage {
        from_role: from_role.into(),
        ts: Utc::now() - chrono::Duration::seconds(seconds_ago),
    }
}

#[test]
fn evidence_check_passes_when_all_required_roles_broadcast_within_window() {
    let workflow = bundled_default();
    let now = Utc::now();
    let messages = vec![msg("designer", 60), msg("dev", 120)];
    assert!(check_evidence_present(&workflow, "accept", "pm", &messages, now).is_ok());
}

#[test]
fn evidence_check_fails_when_designer_verdict_missing() {
    let workflow = bundled_default();
    let now = Utc::now();
    let messages = vec![msg("dev", 60)];
    let err =
        check_evidence_present(&workflow, "accept", "pm", &messages, now).expect_err("missing");
    match err {
        GateError::GateMismatch {
            missing_evidence,
            class,
            ..
        } => {
            assert_eq!(missing_evidence, vec!["designer-verdict".to_string()]);
            assert_eq!(class, GateClass::TeamVerifiable);
        }
        other => panic!("expected GateMismatch, got {other:?}"),
    }
}

#[test]
fn evidence_check_fails_when_dev_verdict_missing() {
    let workflow = bundled_default();
    let now = Utc::now();
    let messages = vec![msg("designer", 60)];
    let err =
        check_evidence_present(&workflow, "accept", "pm", &messages, now).expect_err("missing");
    if let GateError::GateMismatch {
        missing_evidence, ..
    } = err
    {
        assert_eq!(missing_evidence, vec!["dev-verdict".to_string()]);
    } else {
        panic!("expected GateMismatch");
    }
}

#[test]
fn evidence_check_fails_when_both_verdicts_missing() {
    let workflow = bundled_default();
    let now = Utc::now();
    let err = check_evidence_present(&workflow, "accept", "pm", &[], now).expect_err("missing");
    if let GateError::GateMismatch {
        missing_evidence, ..
    } = err
    {
        assert_eq!(
            missing_evidence,
            vec!["designer-verdict".to_string(), "dev-verdict".to_string()]
        );
    } else {
        panic!("expected GateMismatch");
    }
}

#[test]
fn evidence_check_fails_when_designer_verdict_is_stale_beyond_window() {
    let workflow = bundled_default();
    let now = Utc::now();
    let messages = vec![msg("designer", 601), msg("dev", 60)];
    let err = check_evidence_present(&workflow, "accept", "pm", &messages, now)
        .expect_err("stale designer");
    if let GateError::GateMismatch {
        missing_evidence, ..
    } = err
    {
        assert_eq!(missing_evidence, vec!["designer-verdict".to_string()]);
    } else {
        panic!("expected GateMismatch");
    }
}

#[test]
fn evidence_check_passes_for_transitions_with_no_evidence_required() {
    let workflow = bundled_default();
    let now = Utc::now();
    assert!(check_evidence_present(&workflow, "pickup", "dev", &[], now).is_ok());
    assert!(check_evidence_present(&workflow, "deliver", "dev", &[], now).is_ok());
}

#[test]
fn evidence_check_returns_transition_undefined_when_missing() {
    let workflow = WorkflowConfig {
        section: vec![],
        state: vec![],
        transition: vec![],
    };
    let err =
        check_evidence_present(&workflow, "accept", "pm", &[], Utc::now()).expect_err("undefined");
    assert!(matches!(err, GateError::TransitionUndefined { .. }));
}

#[test]
fn gate_mismatch_json_includes_class_and_missing_evidence_fields() {
    let err = GateError::GateMismatch {
        actor_role: "pm".into(),
        attempted_transition: "accept".into(),
        class: GateClass::TeamVerifiable,
        missing_evidence: vec!["designer-verdict".into()],
    };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "workflow-gate-mismatch");
    assert_eq!(json["actor_role"], "pm");
    assert_eq!(json["attempted_transition"], "accept");
    assert_eq!(json["class"], "team-verifiable");
    assert_eq!(json["missing_evidence"][0], "designer-verdict");
}
