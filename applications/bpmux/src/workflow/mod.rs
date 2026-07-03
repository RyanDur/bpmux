pub mod bundled_default;
pub mod config;
pub mod gate;
pub mod loader;
pub mod stage_id;
pub mod verdicts;

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod gate_tests;
#[cfg(test)]
mod loader_tests;
#[cfg(test)]
mod verdict_persistence;

pub use config::{GateClass, Role, Section, State, Transition, WorkflowConfig};
pub use gate::{
    CiSnapshot, EvidenceMessage, GateError, PriorFire, VerdictState, check_ci_green,
    check_evidence_present, check_role_allowed, check_self_mutation_refused,
    check_target_state_valid, enforce_role_allowed, resolve_transition, verdict_freshness,
};
pub use loader::{
    CANONICAL_SECTIONS, WorkflowLoadError, current_workflow, install_workflow,
    load_project_root_or_cwd, load_workflow, validate_section_skeleton, workflow_path,
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
