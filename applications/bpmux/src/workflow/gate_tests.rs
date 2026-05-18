#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::Utc;

use super::bundled_default::bundled_default;
use super::config::{GateClass, Transition, WorkflowConfig};
use super::gate::{
    EvidenceMessage, GateError, PriorFire, check_evidence_present, check_role_allowed,
    check_self_mutation_refused, check_target_state_valid, resolve_transition,
};

fn accept_team_verifiable() -> Transition {
    bundled_default()
        .transition_for("accept", GateClass::TeamVerifiable)
        .cloned()
        .expect("bundled-default has team-verifiable accept")
}

fn accept_owner_walk() -> Transition {
    bundled_default()
        .transition_for("accept", GateClass::OwnerWalk)
        .cloned()
        .expect("bundled-default has owner-walk accept")
}

#[test]
fn bundled_default_allows_ryan_to_accept_team_verifiable() {
    assert!(check_role_allowed(&accept_team_verifiable(), "ryan").is_ok());
}

#[test]
fn bundled_default_allows_pm_to_accept_team_verifiable() {
    assert!(check_role_allowed(&accept_team_verifiable(), "pm").is_ok());
}

#[test]
fn bundled_default_refuses_designer_to_accept_team_verifiable() {
    let err =
        check_role_allowed(&accept_team_verifiable(), "designer").expect_err("designer refused");
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
fn bundled_default_refuses_dev_to_accept_team_verifiable() {
    assert!(matches!(
        check_role_allowed(&accept_team_verifiable(), "dev"),
        Err(GateError::RoleNotAllowed { .. })
    ));
}

#[test]
fn owner_walk_accept_allows_ryan_only() {
    let owner_walk = accept_owner_walk();
    assert!(check_role_allowed(&owner_walk, "ryan").is_ok());
    assert!(matches!(
        check_role_allowed(&owner_walk, "pm"),
        Err(GateError::RoleNotAllowed { .. })
    ));
    assert!(matches!(
        check_role_allowed(&owner_walk, "dev"),
        Err(GateError::RoleNotAllowed { .. })
    ));
}

#[test]
fn resolve_transition_returns_team_verifiable_when_class_team_verifiable() {
    let workflow = bundled_default();
    let t = resolve_transition(&workflow, "accept", GateClass::TeamVerifiable).unwrap();
    assert_eq!(t.gate_class, GateClass::TeamVerifiable);
    assert_eq!(t.required_evidence_roles, vec!["designer", "dev"]);
}

#[test]
fn resolve_transition_returns_owner_walk_when_class_owner_walk() {
    let workflow = bundled_default();
    let t = resolve_transition(&workflow, "accept", GateClass::OwnerWalk).unwrap();
    assert_eq!(t.gate_class, GateClass::OwnerWalk);
    assert!(t.required_evidence_roles.is_empty());
}

#[test]
fn resolve_transition_falls_back_to_class_none_when_no_class_match() {
    let workflow = WorkflowConfig {
        section: vec![],
        state: vec![],
        transition: vec![Transition {
            id: "deliver".into(),
            from: "in-progress".into(),
            to: "desk-check".into(),
            actor_roles: vec!["dev".into()],
            gate_class: GateClass::None,
            evidence_window_seconds: 0,
            required_evidence_roles: vec![],
        }],
    };
    let t = resolve_transition(&workflow, "deliver", GateClass::TeamVerifiable).unwrap();
    assert_eq!(t.gate_class, GateClass::None);
}

#[test]
fn resolve_transition_returns_undefined_when_id_missing() {
    let workflow = WorkflowConfig {
        section: vec![],
        state: vec![],
        transition: vec![],
    };
    let err = resolve_transition(&workflow, "accept", GateClass::TeamVerifiable)
        .expect_err("undefined transition");
    assert!(matches!(err, GateError::TransitionUndefined { .. }));
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
    let now = Utc::now();
    let messages = vec![msg("designer", 60), msg("dev", 120)];
    assert!(check_evidence_present(&accept_team_verifiable(), "pm", &messages, now).is_ok());
}

#[test]
fn evidence_check_fails_when_designer_verdict_missing() {
    let now = Utc::now();
    let messages = vec![msg("dev", 60)];
    let err = check_evidence_present(&accept_team_verifiable(), "pm", &messages, now)
        .expect_err("missing");
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
    let now = Utc::now();
    let messages = vec![msg("designer", 60)];
    let err = check_evidence_present(&accept_team_verifiable(), "pm", &messages, now)
        .expect_err("missing");
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
    let now = Utc::now();
    let err =
        check_evidence_present(&accept_team_verifiable(), "pm", &[], now).expect_err("missing");
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
    let now = Utc::now();
    let messages = vec![msg("designer", 601), msg("dev", 60)];
    let err = check_evidence_present(&accept_team_verifiable(), "pm", &messages, now)
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
fn owner_walk_accept_passes_with_no_team_evidence() {
    let now = Utc::now();
    let owner_walk = accept_owner_walk();
    assert!(check_evidence_present(&owner_walk, "ryan", &[], now).is_ok());
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
    let t = accept_team_verifiable();
    assert!(check_target_state_valid(&t, "delivered").is_ok());
    assert!(check_target_state_valid(&t, "in-progress").is_ok());
}

#[test]
fn target_state_valid_refuses_when_already_in_target_state() {
    let err = check_target_state_valid(&accept_team_verifiable(), "accepted")
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
