pub(super) fn enforce_move_authorization(role: &str) -> Result<(), String> {
    let actor_role = role;
    let workflow = workflow::current_workflow();
    let transition =
        workflow::resolve_transition(workflow, "move-with-position", workflow::GateClass::None)
            .map_err(|e| e.as_json_string())?;
    workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_name().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())
}
