#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::Utc;

use super::bundled_default::bundled_default;
use super::config::{GateClass, Transition, WorkflowConfig};
use super::gate::{
    CiSnapshot, EvidenceMessage, GateError, PriorFire, check_ci_green, check_evidence_present,
    check_role_allowed, check_self_mutation_refused, check_target_state_valid, resolve_transition,
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

fn accept_dev_verifiable() -> Transition {
    bundled_default()
        .transition_for("accept", GateClass::DevVerifiable)
        .cloned()
        .expect("bundled-default has dev-verifiable accept (chore-ownership-triad)")
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
fn dev_verifiable_accept_allows_dev_only() {
    let dev_verifiable = accept_dev_verifiable();
    assert!(check_role_allowed(&dev_verifiable, "dev").is_ok());
    assert!(matches!(
        check_role_allowed(&dev_verifiable, "pm"),
        Err(GateError::RoleNotAllowed { .. })
    ));
    assert!(matches!(
        check_role_allowed(&dev_verifiable, "ryan"),
        Err(GateError::RoleNotAllowed { .. })
    ));
    assert!(matches!(
        check_role_allowed(&dev_verifiable, "designer"),
        Err(GateError::RoleNotAllowed { .. })
    ));
}

#[test]
fn dev_verifiable_accept_requires_designer_evidence() {
    let dev_verifiable = accept_dev_verifiable();
    assert_eq!(dev_verifiable.required_evidence_roles, vec!["designer"]);
}

#[test]
fn resolve_transition_returns_dev_verifiable_when_class_dev_verifiable() {
    let workflow = bundled_default();
    let t = resolve_transition(&workflow, "accept", GateClass::DevVerifiable).unwrap();
    assert_eq!(t.gate_class, GateClass::DevVerifiable);
    assert_eq!(t.actor_roles, vec!["dev"]);
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
        role: vec![],
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
        role: vec![],
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
            assert_eq!(
                missing_evidence,
                vec!["designer-verdict-missing".to_string()]
            );
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
        assert_eq!(missing_evidence, vec!["dev-verdict-missing".to_string()]);
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
            vec![
                "designer-verdict-missing".to_string(),
                "dev-verdict-missing".to_string()
            ]
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
        assert_eq!(
            missing_evidence,
            vec!["designer-verdict-expired".to_string()]
        );
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
        verdict_state: None,
    };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "workflow-gate-mismatch");
    assert_eq!(json["actor_role"], "pm");
    assert_eq!(json["attempted_transition"], "accept");
    assert_eq!(json["class"], "team-verifiable");
    assert_eq!(json["missing_evidence"][0], "designer-verdict");
}

#[test]
fn evidence_check_surfaces_verdict_state_per_role_when_missing_or_stale_1461() {
    let now = Utc::now();
    let messages = vec![msg("designer", 60), msg("dev", 800)];
    let err = check_evidence_present(&accept_team_verifiable(), "pm", &messages, now)
        .expect_err("dev stale");
    match err {
        GateError::GateMismatch {
            missing_evidence,
            verdict_state,
            ..
        } => {
            assert_eq!(missing_evidence, vec!["dev-verdict-expired".to_string()]);
            let state = verdict_state.expect("verdict_state populated");
            let designer = state.get("designer").expect("designer entry");
            assert!(designer.cast_at.is_some(), "designer cast_at populated");
            assert!(designer.expired_at.is_some(), "designer expired_at derived");
            assert!(
                designer.stale_age_secs.is_none(),
                "designer verdict in-window: stale_age_secs absent"
            );
            let dev = state.get("dev").expect("dev entry");
            assert!(dev.cast_at.is_some(), "dev cast_at populated");
            assert!(
                dev.stale_age_secs.is_some_and(|secs| secs > 0),
                "dev verdict stale: stale_age_secs positive"
            );
        }
        other => panic!("expected GateMismatch, got {other:?}"),
    }
}

#[test]
fn evidence_check_distinguishes_missing_from_stale_in_verdict_state_1461() {
    let now = Utc::now();
    let messages = vec![msg("designer", 9999)];
    let err = check_evidence_present(&accept_team_verifiable(), "pm", &messages, now)
        .expect_err("designer stale + dev missing");
    match err {
        GateError::GateMismatch {
            missing_evidence,
            verdict_state,
            ..
        } => {
            assert!(missing_evidence.contains(&"designer-verdict-expired".to_string()));
            assert!(missing_evidence.contains(&"dev-verdict-missing".to_string()));
            let state = verdict_state.expect("verdict_state populated");
            assert!(
                state.get("designer").and_then(|v| v.cast_at).is_some(),
                "designer cast_at populated (stale)"
            );
            assert!(
                state.get("dev").and_then(|v| v.cast_at).is_none(),
                "dev cast_at null (missing)"
            );
        }
        other => panic!("expected GateMismatch, got {other:?}"),
    }
}

#[test]
fn evidence_check_no_verdict_state_when_class_is_none_or_no_required_roles_1461() {
    let now = Utc::now();
    let owner_walk = accept_owner_walk();
    let result = check_evidence_present(&owner_walk, "ryan", &[], now);
    assert!(result.is_ok());
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

fn pass_desk_check_transition() -> Transition {
    bundled_default()
        .transition_for("pass-desk-check", GateClass::TeamVerifiable)
        .cloned()
        .or_else(|| {
            bundled_default()
                .transition_for("pass-desk-check", GateClass::None)
                .cloned()
        })
        .unwrap_or(Transition {
            id: "pass-desk-check".to_string(),
            from: "delivered".to_string(),
            to: "delivered".to_string(),
            actor_roles: vec!["designer".to_string()],
            gate_class: GateClass::None,
            required_evidence_roles: Vec::new(),
            evidence_window_seconds: 600,
        })
}

#[test]
fn ci_green_passes_on_success_conclusion() {
    let snap = CiSnapshot {
        conclusion: "success".to_string(),
        head_sha: "abc123".to_string(),
    };
    assert!(check_ci_green(&pass_desk_check_transition(), Some(&snap)).is_ok());
}

#[test]
fn ci_green_refuses_on_failure_conclusion() {
    let snap = CiSnapshot {
        conclusion: "failure".to_string(),
        head_sha: "deadbeef".to_string(),
    };
    let err = check_ci_green(&pass_desk_check_transition(), Some(&snap))
        .expect_err("failure CI must refuse");
    match err {
        GateError::CiNotGreen {
            attempted_transition,
            ci_conclusion,
            head_sha,
            green_head_check_path,
        } => {
            assert_eq!(attempted_transition, "pass-desk-check");
            assert_eq!(ci_conclusion, "failure");
            assert_eq!(head_sha, "deadbeef");
            assert_eq!(green_head_check_path, "neither");
        }
        other => panic!("expected CiNotGreen, got {other:?}"),
    }
}

#[test]
fn ci_green_refuses_on_in_progress_conclusion() {
    let snap = CiSnapshot {
        conclusion: "in_progress".to_string(),
        head_sha: "wip".to_string(),
    };
    let err = check_ci_green(&pass_desk_check_transition(), Some(&snap))
        .expect_err("in_progress CI must refuse");
    assert!(
        matches!(err, GateError::CiNotGreen { ci_conclusion, .. } if ci_conclusion == "in_progress")
    );
}

#[test]
fn ci_green_quota_exhausted_is_exception_class() {
    let snap = CiSnapshot {
        conclusion: "quota_exhausted".to_string(),
        head_sha: "quota".to_string(),
    };
    assert!(
        check_ci_green(&pass_desk_check_transition(), Some(&snap)).is_ok(),
        "quota_exhausted is an exception class (sister to pre-accept-ci-check skill)"
    );
}

#[test]
fn ci_green_refuses_when_snapshot_absent() {
    let err = check_ci_green(&pass_desk_check_transition(), None)
        .expect_err("absent snapshot must refuse");
    assert!(
        matches!(err, GateError::CiNotGreen { ci_conclusion, head_sha, .. } if ci_conclusion == "unknown" && head_sha.is_empty())
    );
}

#[test]
fn ci_not_green_json_shape() {
    let err = GateError::CiNotGreen {
        attempted_transition: "pass-desk-check".into(),
        ci_conclusion: "in_progress".into(),
        head_sha: "abc123".into(),
        green_head_check_path: "neither".into(),
    };
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "workflow-ci-not-green");
    assert_eq!(json["attempted_transition"], "pass-desk-check");
    assert_eq!(json["ci_conclusion"], "in_progress");
    assert_eq!(json["head_sha"], "abc123");
    assert_eq!(json["green_head_check_path"], "neither");
}
