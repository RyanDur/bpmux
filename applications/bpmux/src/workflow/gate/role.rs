use super::super::config::Transition;
use super::GateError;

pub fn check_role_allowed(transition: &Transition, actor_role: &str) -> Result<(), GateError> {
    if transition.actor_roles.iter().any(|r| r == actor_role) {
        Ok(())
    } else {
        Err(GateError::RoleNotAllowed {
            actor_role: actor_role.to_string(),
            attempted_transition: transition.id.clone(),
            allowed_roles: transition.actor_roles.clone(),
        })
    }
}

pub fn enforce_role_allowed(
    transition: &Transition,
    actor_role: &str,
    owner_role: &str,
) -> Result<(), GateError> {
    if !owner_role.is_empty() && actor_role == owner_role {
        return Ok(());
    }
    check_role_allowed(transition, actor_role)
}

pub fn enforce_stage_judge(
    workflow: &super::super::config::WorkflowConfig,
    stage_id: &str,
    actor_role: &str,
    actor_agent_id: &str,
) -> Result<(), GateError> {
    let judges = workflow.judges_of(stage_id);
    if judges.is_empty() {
        return Ok(());
    }
    let owner_role = workflow.owner_role_id().unwrap_or("");
    if !owner_role.is_empty() && actor_role == owner_role {
        return Ok(());
    }
    if judges
        .iter()
        .any(|judge| judge == actor_role || judge == actor_agent_id)
    {
        return Ok(());
    }
    Err(GateError::RoleNotAllowed {
        actor_role: actor_role.to_string(),
        attempted_transition: format!("judge:{stage_id}"),
        allowed_roles: judges.to_vec(),
    })
}
