use crate::bridge::tools_stories::role_from_name;
use crate::session::identity::{
    add_value, default_identity_dir, load_identity, save_identity,
};

use super::helpers::{identity_error, identity_success};

/// `add_value` — create a new top-level value on the caller's own
/// identity file. Role-derived (agents can only mutate their own).
pub(in crate::bridge) async fn handle_add_value(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
) -> Result<String, String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing 'name' argument")?;
    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .ok_or("missing 'description' argument")?;
    let role = role_from_name(agent_name);
    let base = default_identity_dir().map_err(|e| format!("identity dir: {e}"))?;
    let mut index = load_identity(&base, role);
    if let Err(e) = add_value(&mut index, name, description) {
        return Err(identity_error(&e));
    }
    save_identity(&base, role, &index).map_err(|e| format!("save: {e}"))?;
    Ok(identity_success(&index))
}
