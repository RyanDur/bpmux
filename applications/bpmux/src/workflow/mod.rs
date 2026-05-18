//! Workflow config substrate (#1384 Epic, #1385 first slice).
//!
//! Loads `<project-root>/.bpmux/workflow.toml` if present; falls back
//! to the bundled-default otherwise. Hard-fails on malformed TOML —
//! NEVER silent-fallback to bundled-default when the file exists but
//! is invalid (sister-consistency with #1394 section-skeleton hard-fail
//! per `feedback_substrate_enforce_canon_shape_not_canon_existence`).
//!
//! Slice 1 scope: sections + states + loader + bundled-default +
//! structured error. Slice 2+ adds transitions, role-resolver, and
//! accept_story consumer-wiring.

pub mod bundled_default;
pub mod config;
pub mod gate;
pub mod loader;

#[cfg(test)]
mod gate_tests;
#[cfg(test)]
mod loader_tests;

pub use config::{GateClass, Section, State, Transition, WorkflowConfig, gate_class_from_labels};
pub use gate::{
    EvidenceMessage, GateError, PriorFire, check_evidence_present, check_role_allowed,
    check_self_mutation_refused, check_target_state_valid, resolve_transition,
};
pub use loader::{
    WorkflowLoadError, current_workflow, install_workflow, load_project_root_or_cwd, load_workflow,
    workflow_path,
};
