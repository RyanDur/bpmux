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
