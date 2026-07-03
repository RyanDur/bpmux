#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod gate_class_from_acceptor_and_story_type_tests_1460 {
    use crate::product_management::story::{Acceptor, StoryKind};
    use crate::workflow::config::*;

    #[test]
    fn acceptor_owner_returns_owner_walk() {
        assert_eq!(
            gate_class_from_acceptor_and_story_type(Some(Acceptor::Owner), StoryKind::Feature),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn acceptor_team_returns_team_verifiable() {
        assert_eq!(
            gate_class_from_acceptor_and_story_type(Some(Acceptor::Team), StoryKind::Feature),
            GateClass::TeamVerifiable
        );
    }

    #[test]
    fn acceptor_dev_returns_dev_verifiable() {
        assert_eq!(
            gate_class_from_acceptor_and_story_type(Some(Acceptor::Dev), StoryKind::Chore),
            GateClass::DevVerifiable
        );
    }

    #[test]
    fn acceptor_overrides_story_type() {
        assert_eq!(
            gate_class_from_acceptor_and_story_type(Some(Acceptor::Dev), StoryKind::Feature),
            GateClass::DevVerifiable
        );
    }

    #[test]
    fn falls_back_to_story_type_feature_owner_walk() {
        assert_eq!(
            gate_class_from_acceptor_and_story_type(None, StoryKind::Feature),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn falls_back_to_story_type_bug_team_verifiable() {
        assert_eq!(
            gate_class_from_acceptor_and_story_type(None, StoryKind::Bug),
            GateClass::TeamVerifiable
        );
    }

    #[test]
    fn falls_back_to_story_type_chore_dev_verifiable() {
        assert_eq!(
            gate_class_from_acceptor_and_story_type(None, StoryKind::Chore),
            GateClass::DevVerifiable
        );
    }

    #[test]
    fn acceptor_only_none_defaults_team_verifiable() {
        assert_eq!(gate_class_from_acceptor(None), GateClass::TeamVerifiable);
    }

    #[test]
    fn acceptor_only_owner_returns_owner_walk() {
        assert_eq!(
            gate_class_from_acceptor(Some(Acceptor::Owner)),
            GateClass::OwnerWalk
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod role_resolver_tests {
    use crate::workflow::config::*;

    fn workflow_with_roles(roles: Vec<Role>) -> WorkflowConfig {
        WorkflowConfig {
            section: vec![],
            state: vec![],
            transition: vec![],
            role: roles,
        }
    }

    #[test]
    fn role_id_for_purpose_returns_matching_role() {
        let workflow = workflow_with_roles(vec![Role {
            id: "alice".into(),
            purposes: vec!["owner".into()],
        }]);
        assert_eq!(workflow.role_id_for_purpose("owner"), Some("alice"));
    }

    #[test]
    fn role_id_for_purpose_returns_none_when_no_role_matches() {
        let workflow = workflow_with_roles(vec![Role {
            id: "alice".into(),
            purposes: vec!["owner".into()],
        }]);
        assert_eq!(workflow.role_id_for_purpose("pm"), None);
    }

    #[test]
    fn role_can_carry_multiple_purposes() {
        let workflow = workflow_with_roles(vec![Role {
            id: "alice".into(),
            purposes: vec!["pm".into(), "lead".into()],
        }]);
        assert_eq!(workflow.role_id_for_purpose("pm"), Some("alice"));
        assert_eq!(workflow.role_id_for_purpose("lead"), Some("alice"));
    }

    #[test]
    fn role_ids_for_purpose_collects_all_roles_carrying_it() {
        let workflow = workflow_with_roles(vec![
            Role {
                id: "designer".into(),
                purposes: vec!["designer".into(), "iteration-reviewer".into()],
            },
            Role {
                id: "lead".into(),
                purposes: vec!["dev".into(), "iteration-reviewer".into()],
            },
            Role {
                id: "pm".into(),
                purposes: vec!["pm".into()],
            },
        ]);
        assert_eq!(
            workflow.role_ids_for_purpose("iteration-reviewer"),
            vec!["designer", "lead"]
        );
    }

    #[test]
    fn role_ids_for_purpose_empty_when_none_carry_it() {
        let workflow = workflow_with_roles(vec![Role {
            id: "pm".into(),
            purposes: vec!["pm".into()],
        }]);
        assert!(
            workflow
                .role_ids_for_purpose("iteration-reviewer")
                .is_empty()
        );
    }

    #[test]
    fn owner_role_id_resolves_via_owner_purpose() {
        let workflow = workflow_with_roles(vec![
            Role {
                id: "alice".into(),
                purposes: vec!["owner".into()],
            },
            Role {
                id: "bob".into(),
                purposes: vec!["dev".into()],
            },
        ]);
        assert_eq!(workflow.owner_role_id(), Some("alice"));
    }

    #[test]
    fn owner_role_id_returns_none_when_no_owner_purpose() {
        let workflow = workflow_with_roles(vec![Role {
            id: "bob".into(),
            purposes: vec!["dev".into()],
        }]);
        assert_eq!(workflow.owner_role_id(), None);
    }

    #[test]
    fn bundled_default_resolves_canonical_purposes_to_canonical_ids() {
        let workflow = crate::workflow::bundled_default::bundled_default();
        assert_eq!(workflow.role_id_for_purpose("owner"), Some("owner"));
        assert_eq!(workflow.role_id_for_purpose("pm"), Some("pm"));
        assert_eq!(workflow.role_id_for_purpose("designer"), Some("designer"));
        assert_eq!(workflow.role_id_for_purpose("dev"), Some("dev"));
        assert_eq!(workflow.owner_role_id(), Some("owner"));
    }

    #[test]
    fn bundled_default_designates_designer_as_iteration_reviewer() {
        let workflow = crate::workflow::bundled_default::bundled_default();
        assert_eq!(
            workflow.role_ids_for_purpose("iteration-reviewer"),
            vec!["designer"]
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod board_middle_stage_ids_tests {
    use crate::workflow::{board_middle_stage_ids, bundled_default::bundled_default};

    #[test]
    fn middle_stages_derive_from_the_in_work_section_in_config_order() {
        let ids: Vec<String> = board_middle_stage_ids(&bundled_default())
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        assert_eq!(
            ids,
            vec!["in-progress".to_string(), "desk-check".to_string()],
            "the board's configurable middle is exactly the in-work section, in config order"
        );
    }
}
