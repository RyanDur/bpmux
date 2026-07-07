pub mod bundled_default;
pub mod config;
pub mod gate;
pub mod loader;
pub mod stage_id;

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod gate_tests;
#[cfg(test)]
mod loader_tests;

pub use config::{GateClass, Role, Section, State, Transition, WorkflowConfig};
pub use gate::{
    CiSnapshot, EvidenceMessage, GateError, PriorFire, VerdictState, check_ci_green,
    check_evidence_present, check_role_allowed, check_self_mutation_refused,
    check_target_state_valid, enforce_role_allowed, enforce_stage_judge, resolve_transition,
    verdict_freshness,
};
pub use loader::{
    CANONICAL_SECTIONS, WorkflowLoadError, current_workflow, install_workflow,
    load_project_root_or_cwd, load_workflow, load_workflow_file, load_workflow_from_process_env,
    load_workflow_with_override, validate_section_skeleton, workflow_path,
};
pub use stage_id::StageId;

pub const IN_WORK_SECTION: &str = "in-work";

pub fn board_middle_stage_ids(config: &WorkflowConfig) -> Vec<StageId> {
    config
        .state
        .iter()
        .filter(|s| s.section == IN_WORK_SECTION)
        .map(|s| StageId::from(s.id.as_str()))
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
pub mod test_support {
    use std::path::Path;

    use super::config::WorkflowConfig;

    pub fn repo_workflow() -> WorkflowConfig {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest
            .parent()
            .and_then(Path::parent)
            .expect("crate lives two levels under the repo root");
        super::loader::load_workflow(repo_root)
            .expect("the repo's .bpmux/workflow.toml is the team config and must load")
    }
}
