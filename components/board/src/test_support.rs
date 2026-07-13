#![allow(clippy::expect_used)]

use std::path::Path;

use workflow::WorkflowConfig;

pub fn repo_workflow() -> WorkflowConfig {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two levels under the repo root");
    workflow::load_workflow(repo_root)
        .expect("the repo's .bpmux/workflow.toml is the team config and must load")
}
