pub(crate) fn role_from_name(name: &str) -> &str {
    name.split(':').next().unwrap_or(name)
}

pub(crate) fn require_role(
    agent_name: &str,
    allowed: &[&str],
    operation: &str,
) -> Result<(), String> {
    let role = role_from_name(agent_name);
    if let Some(owner) = crate::workflow::current_workflow().owner_role_name()
        && !owner.is_empty()
        && role == owner
    {
        return Ok(());
    }
    if allowed.contains(&role) {
        Ok(())
    } else {
        Err(format!(
            "{operation} is restricted to {}. ask {} to {operation}.",
            allowed.join(" or "),
            allowed[0],
        ))
    }
}

pub(crate) fn require_purpose(
    agent_name: &str,
    purposes: &[&str],
    operation: &str,
) -> Result<(), String> {
    let workflow = crate::workflow::current_workflow();
    let allowed: Vec<String> = purposes
        .iter()
        .map(|p| {
            workflow
                .role_name_for_purpose(p)
                .map(str::to_string)
                .unwrap_or_else(|| (*p).to_string())
        })
        .collect();
    let allowed_refs: Vec<&str> = allowed.iter().map(String::as_str).collect();
    require_role(agent_name, &allowed_refs, operation)
}

pub(crate) fn require_purpose_any(
    agent_name: &str,
    purpose: &str,
    operation: &str,
) -> Result<(), String> {
    let workflow = crate::workflow::current_workflow();
    let allowed = workflow.role_names_for_purpose(purpose);
    if allowed.is_empty() {
        return Err(format!("{operation} has no configured {purpose} role"));
    }
    require_role(agent_name, &allowed, operation)
}
