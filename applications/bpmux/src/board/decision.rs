use crate::product_management::story::{Verdict, Work};
use crate::workflow::{StageId, WorkflowConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub coarse: Work,
    pub placement: Option<StageId>,
}

fn state_position(id: &str, workflow: &WorkflowConfig) -> Option<usize> {
    workflow.state.iter().position(|s| s.id == id)
}

fn next_state_id_in_order(from_id: &str, workflow: &WorkflowConfig) -> Option<String> {
    let pos = state_position(from_id, workflow)?;
    workflow.state.get(pos + 1).map(|s| s.id.clone())
}

fn backward_target_id(from_id: &str, workflow: &WorkflowConfig) -> Option<String> {
    let from_pos = state_position(from_id, workflow)?;
    workflow
        .transition
        .iter()
        .find(|t| {
            t.from == from_id
                && state_position(&t.to, workflow).is_some_and(|to_pos| to_pos < from_pos)
        })
        .map(|t| t.to.clone())
}

fn default_fail_target_id(from_id: &str, workflow: &WorkflowConfig) -> Option<String> {
    let middle = crate::workflow::board_middle_stage_ids(workflow);
    let stage_index = middle.iter().position(|stage| stage.as_str() == from_id)?;
    match stage_index {
        0 => Some(from_id.to_string()),
        n => Some(middle[n - 1].as_str().to_string()),
    }
}

fn fail_target_id(from_id: &str, workflow: &WorkflowConfig) -> Option<String> {
    workflow
        .fail_to_of(from_id)
        .map(str::to_string)
        .or_else(|| backward_target_id(from_id, workflow))
        .or_else(|| default_fail_target_id(from_id, workflow))
}

pub fn advance(verdict: &Verdict, workflow: &WorkflowConfig) -> Option<Decision> {
    let target_id = match verdict {
        Verdict::Succeed { step } => next_state_id_in_order(step.as_str(), workflow)?,
        Verdict::Fail { step } => fail_target_id(step.as_str(), workflow)?,
    };
    let middle = crate::workflow::board_middle_stage_ids(workflow);
    let coarse = if middle.iter().any(|stage| stage.as_str() == target_id) {
        Work::InProgress
    } else {
        Work::parse(&target_id)?
    };
    Some(Decision {
        coarse,
        placement: super::placement_from_target(&target_id, &middle),
    })
}
