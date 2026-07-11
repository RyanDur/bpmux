use crate::bridge::tools_stories::role_from_name;
use memory::identity::{default_identity_dir, load_identity, save_identity, update_principle};

use super::helpers::{identity_error, identity_success};

pub(in crate::bridge) async fn handle_update_principle(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
) -> Result<String, String> {
    let value_name = args
        .get("value_name")
        .and_then(|v| v.as_str())
        .ok_or("missing 'value_name' argument")?;
    let old = args
        .get("old")
        .and_then(|v| v.as_str())
        .ok_or("missing 'old' argument")?;
    let new = args
        .get("new")
        .and_then(|v| v.as_str())
        .ok_or("missing 'new' argument")?;
    let role = role_from_name(agent_name);
    let base = default_identity_dir().map_err(|e| format!("identity dir: {e}"))?;
    let mut index = load_identity(&base, role);
    if let Err(e) = update_principle(&mut index, value_name, old, new) {
        return Err(identity_error(&e));
    }
    save_identity(&base, role, &index).map_err(|e| format!("save: {e}"))?;
    Ok(identity_success(&index))
}
