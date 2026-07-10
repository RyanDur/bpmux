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
    fn role_name_for_purpose_returns_matching_role() {
        let workflow = workflow_with_roles(vec![Role {
            name: "alice".into(),
            purposes: vec!["owner".into()],
        }]);
        assert_eq!(workflow.role_name_for_purpose("owner"), Some("alice"));
    }

    #[test]
    fn role_name_for_purpose_returns_none_when_no_role_matches() {
        let workflow = workflow_with_roles(vec![Role {
            name: "alice".into(),
            purposes: vec!["owner".into()],
        }]);
        assert_eq!(workflow.role_name_for_purpose("pm"), None);
    }

    #[test]
    fn role_can_carry_multiple_purposes() {
        let workflow = workflow_with_roles(vec![Role {
            name: "alice".into(),
            purposes: vec!["pm".into(), "lead".into()],
        }]);
        assert_eq!(workflow.role_name_for_purpose("pm"), Some("alice"));
        assert_eq!(workflow.role_name_for_purpose("lead"), Some("alice"));
    }

    #[test]
    fn role_names_for_purpose_collects_all_roles_carrying_it() {
        let workflow = workflow_with_roles(vec![
            Role {
                name: "designer".into(),
                purposes: vec!["designer".into(), "iteration-reviewer".into()],
            },
            Role {
                name: "lead".into(),
                purposes: vec!["dev".into(), "iteration-reviewer".into()],
            },
            Role {
                name: "pm".into(),
                purposes: vec!["pm".into()],
            },
        ]);
        assert_eq!(
            workflow.role_names_for_purpose("iteration-reviewer"),
            vec!["designer", "lead"]
        );
    }

    #[test]
    fn role_names_for_purpose_empty_when_none_carry_it() {
        let workflow = workflow_with_roles(vec![Role {
            name: "pm".into(),
            purposes: vec!["pm".into()],
        }]);
        assert!(
            workflow
                .role_names_for_purpose("iteration-reviewer")
                .is_empty()
        );
    }

    #[test]
    fn owner_role_name_resolves_via_owner_purpose() {
        let workflow = workflow_with_roles(vec![
            Role {
                name: "alice".into(),
                purposes: vec!["owner".into()],
            },
            Role {
                name: "bob".into(),
                purposes: vec!["dev".into()],
            },
        ]);
        assert_eq!(workflow.owner_role_name(), Some("alice"));
    }

    #[test]
    fn owner_role_name_returns_none_when_no_owner_purpose() {
        let workflow = workflow_with_roles(vec![Role {
            name: "bob".into(),
            purposes: vec!["dev".into()],
        }]);
        assert_eq!(workflow.owner_role_name(), None);
    }

    #[test]
    fn bundled_default_resolves_canonical_purposes_to_canonical_names() {
        let workflow = crate::workflow::bundled_default::bundled_default();
        assert_eq!(workflow.role_name_for_purpose("owner"), Some("owner"));
        assert_eq!(workflow.role_name_for_purpose("pm"), Some("pm"));
        assert_eq!(workflow.role_name_for_purpose("designer"), Some("designer"));
        assert_eq!(workflow.role_name_for_purpose("dev"), Some("dev"));
        assert_eq!(workflow.owner_role_name(), Some("owner"));
    }

    #[test]
    fn bundled_default_designates_designer_as_iteration_reviewer() {
        let workflow = crate::workflow::bundled_default::bundled_default();
        assert_eq!(
            workflow.role_names_for_purpose("iteration-reviewer"),
            vec!["designer"]
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod board_middle_stage_ids_tests {
    use crate::workflow::{board_middle_stage_ids, bundled_default::bundled_default, test_support};

    #[test]
    fn bundled_default_middle_is_a_single_undivided_in_progress_stage() {
        let ids: Vec<String> = board_middle_stage_ids(&bundled_default())
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        assert_eq!(
            ids,
            vec!["in-progress".to_string()],
            "without a workflow.toml the board is the default three states: ready, one undivided in-progress, acceptance"
        );
    }

    #[test]
    fn middle_stages_derive_from_the_in_work_section_in_config_order() {
        let ids: Vec<String> = board_middle_stage_ids(&test_support::repo_workflow())
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        assert_eq!(
            ids,
            vec!["in-progress".to_string(), "desk-check".to_string()],
            "the board's configurable middle is exactly the configured in-work section, in config order — the repo's own workflow.toml declares the team's two stages"
        );
    }
}
