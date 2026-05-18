#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::bundled_default::bundled_default;
use super::config::{Transition, WorkflowConfig};
use super::gate::{GateError, check_role_allowed};

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
