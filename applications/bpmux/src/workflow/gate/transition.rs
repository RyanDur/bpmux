use super::super::config::{GateClass, Transition, WorkflowConfig};
use super::GateError;

pub fn check_target_state_valid(
    transition: &Transition,
    current_state: &str,
) -> Result<(), GateError> {
    if current_state == transition.to {
        return Err(GateError::TargetStateInvalid {
            attempted_transition: transition.id.clone(),
            current_state: current_state.to_string(),
            expected_state: transition.from.clone(),
        });
    }
    Ok(())
}

pub fn resolve_transition<'a>(
    workflow: &'a WorkflowConfig,
    transition_id: &str,
    class: GateClass,
) -> Result<&'a Transition, GateError> {
    workflow
        .transition_for(transition_id, class)
        .ok_or_else(|| GateError::TransitionUndefined {
            attempted_transition: transition_id.to_string(),
        })
}

#[derive(Debug, Clone)]
pub struct PriorFire {
    pub gate_id: String,
    pub actor_role: String,
}

pub fn check_self_mutation_refused(
    actor_role: &str,
    attempted_transition: &str,
    prior_fires: &[PriorFire],
    refused_prior_gates: &[&str],
) -> Result<(), GateError> {
    for prior in prior_fires {
        if refused_prior_gates.contains(&prior.gate_id.as_str()) && prior.actor_role == actor_role {
            return Err(GateError::SelfMutationRefused {
                actor_role: actor_role.to_string(),
                attempted_transition: attempted_transition.to_string(),
                prior_fire: prior.gate_id.clone(),
            });
        }
    }
    Ok(())
}
