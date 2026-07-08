use crate::product_management::iteration::IterationId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterationLabelParts {
    pub name: String,
    pub position: u32,
}

pub fn iteration_name_from_title(title: &str) -> String {
    let raw = title.strip_prefix("Iteration ").unwrap_or(title).trim();
    raw.split_whitespace().collect::<Vec<_>>().join("-")
}

pub const DEFAULT_FIRST_POSITION: u32 = 100;
pub const DEFAULT_STEP: u32 = 100;

pub fn current_iteration(iterations: &[super::Iteration]) -> Option<&super::Iteration> {
    iterations
        .iter()
        .find(|iteration| !iteration.complete && iteration.is_current)
}

pub fn current_iteration_id(iterations: &[super::Iteration]) -> Option<IterationId> {
    current_iteration(iterations).map(|iteration| iteration.id)
}
