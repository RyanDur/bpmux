#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod gate_class_from_labels_and_story_type_tests_1460 {
    use crate::product_management::story::StoryType;
    use crate::workflow::config::*;

    fn lbl(s: &str) -> String {
        s.to_string()
    }

    #[test]
    fn label_first_priority_awaiting_owner_returns_owner_walk() {
        let labels = vec![lbl("awaiting owner"), lbl("Feature")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn label_first_priority_awaiting_team_returns_team_verifiable() {
        let labels = vec![lbl("awaiting team"), lbl("Feature")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::TeamVerifiable
        );
    }

    #[test]
    fn case_insensitive_label_match() {
        let labels = vec![lbl("Awaiting Team")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::TeamVerifiable
        );
        let labels = vec![lbl("AWAITING OWNER")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Chore),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn falls_back_to_story_type_feature_owner_walk() {
        let labels: Vec<String> = vec![];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn falls_back_to_story_type_bug_team_verifiable() {
        let labels: Vec<String> = vec![];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Bug),
            GateClass::TeamVerifiable
        );
    }

    #[test]
    fn falls_back_to_story_type_chore_dev_verifiable() {
        let labels: Vec<String> = vec![];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Chore),
            GateClass::DevVerifiable
        );
    }

    #[test]
    fn label_first_priority_awaiting_dev_returns_dev_verifiable() {
        let labels = vec![lbl("awaiting dev"), lbl("Chore")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Chore),
            GateClass::DevVerifiable
        );
    }

    #[test]
    fn awaiting_dev_label_overrides_feature_story_type() {
        let labels = vec![lbl("awaiting dev")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::DevVerifiable
        );
    }

    #[test]
    fn legacy_gate_dev_verifiable_label_honored() {
        let labels = vec![lbl("gate:dev-verifiable")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::DevVerifiable
        );
    }

    #[test]
    fn falls_back_to_story_type_epic_owner_walk() {
        let labels: Vec<String> = vec![];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Epic),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn falls_back_to_story_type_iteration_owner_walk() {
        let labels: Vec<String> = vec![];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Iteration),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn legacy_gate_label_still_honored_when_no_awaiting_label() {
        let labels = vec![lbl("gate:owner-walk")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Bug),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn awaiting_label_wins_over_legacy_gate_label() {
        let labels = vec![lbl("gate:team-verifiable"), lbl("awaiting owner")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Bug),
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
