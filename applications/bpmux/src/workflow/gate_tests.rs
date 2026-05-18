#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::Utc;

use super::bundled_default::bundled_default;
use super::config::{GateClass, Transition, WorkflowConfig};
use super::gate::{
    EvidenceMessage, GateError, PriorFire, check_evidence_present, check_role_allowed,
    check_self_mutation_refused, check_target_state_valid,
};

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

#[test]
fn target_state_valid_passes_when_current_is_not_yet_target() {
    let workflow = bundled_default();
    assert!(check_target_state_valid(&workflow, "accept", "delivered").is_ok());
    assert!(check_target_state_valid(&workflow, "accept", "in-progress").is_ok());
}

#[test]
fn target_state_valid_refuses_when_already_in_target_state() {
    let workflow = bundled_default();
    let err = check_target_state_valid(&workflow, "accept", "accepted")
        .expect_err("accepted-already refused");
    match err {
        GateError::TargetStateInvalid {
            attempted_transition,
            current_state,
            expected_state,
        } => {
            assert_eq!(attempted_transition, "accept");
            assert_eq!(current_state, "accepted");
            assert_eq!(expected_state, "delivered");
        }
        other => panic!("expected TargetStateInvalid, got {other:?}"),
    }
}

#[test]
fn target_state_valid_returns_transition_undefined_when_unknown() {
    let workflow = WorkflowConfig {
        section: vec![],
        state: vec![],
        transition: vec![],
    };
    let err = check_target_state_valid(&workflow, "accept", "delivered")
        .expect_err("undefined transition");
    assert!(matches!(err, GateError::TransitionUndefined { .. }));
}

#[test]
fn target_state_invalid_json_shape() {
    let err = GateError::TargetStateInvalid {
        attempted_transition: "accept".into(),
        current_state: "accepted".into(),
        expected_state: "delivered".into(),
    };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "workflow-target-state-invalid");
    assert_eq!(json["current_state"], "accepted");
    assert_eq!(json["expected_state"], "delivered");
}

fn prior(gate_id: &str, actor: &str) -> PriorFire {
    PriorFire {
        gate_id: gate_id.into(),
        actor_role: actor.into(),
    }
}

#[test]
fn self_mutation_refused_when_same_actor_fired_deliver() {
    let prior_fires = vec![prior("deliver", "pm")];
    let err = check_self_mutation_refused("pm", "accept", &prior_fires, &["deliver"])
        .expect_err("same actor refused");
    match err {
        GateError::SelfMutationRefused {
            actor_role,
            attempted_transition,
            prior_fire,
        } => {
            assert_eq!(actor_role, "pm");
            assert_eq!(attempted_transition, "accept");
            assert_eq!(prior_fire, "deliver");
        }
        other => panic!("expected SelfMutationRefused, got {other:?}"),
    }
}

#[test]
fn self_mutation_passes_when_different_actor_fired_deliver() {
    let prior_fires = vec![prior("deliver", "dev")];
    assert!(check_self_mutation_refused("pm", "accept", &prior_fires, &["deliver"]).is_ok());
}

#[test]
fn self_mutation_passes_when_no_prior_fires() {
    assert!(check_self_mutation_refused("pm", "accept", &[], &["deliver"]).is_ok());
}

#[test]
fn self_mutation_refused_when_actor_fired_pass_desk_check() {
    let prior_fires = vec![prior("pass-desk-check", "pm")];
    let err = check_self_mutation_refused(
        "pm",
        "accept",
        &prior_fires,
        &["deliver", "pass-desk-check"],
    )
    .expect_err("pm passed desk-check + cannot self-accept");
    assert!(matches!(err, GateError::SelfMutationRefused { .. }));
}

#[test]
fn self_mutation_ignores_gates_outside_refused_list() {
    let prior_fires = vec![prior("pickup", "pm")];
    assert!(check_self_mutation_refused("pm", "accept", &prior_fires, &["deliver"]).is_ok());
}

#[test]
fn self_mutation_refused_json_shape() {
    let err = GateError::SelfMutationRefused {
        actor_role: "pm".into(),
        attempted_transition: "accept".into(),
        prior_fire: "deliver".into(),
    };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "workflow-self-mutation-refused");
    assert_eq!(json["actor_role"], "pm");
    assert_eq!(json["attempted_transition"], "accept");
    assert_eq!(json["prior_fire"], "deliver");
}
