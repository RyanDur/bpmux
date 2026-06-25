pub mod bundled_default;
pub mod config;
pub mod gate;
pub mod loader;
pub mod verdicts;

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod gate_tests;
#[cfg(test)]
mod loader_tests;
#[cfg(test)]
mod verdict_persistence;

pub use config::{
    GateClass, Role, Section, State, Transition, WorkflowConfig, gate_class_from_labels,
    gate_class_from_labels_and_story_type,
};
pub use gate::{
    CiSnapshot, EvidenceMessage, GateError, PriorFire, VerdictState, check_ci_green,
    check_evidence_present, check_role_allowed, check_self_mutation_refused,
    check_target_state_valid, enforce_role_allowed, resolve_transition, verdict_freshness,
};
pub use loader::{
    CANONICAL_SECTIONS, WorkflowLoadError, current_workflow, install_workflow,
    load_project_root_or_cwd, load_workflow, validate_section_skeleton, workflow_path,
};
