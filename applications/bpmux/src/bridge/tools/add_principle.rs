use crate::bridge::tools_stories::role_from_name;
use crate::memory::identity::{add_principle, default_identity_dir, load_identity, save_identity};

use super::helpers::{identity_error, identity_success};

pub(in crate::bridge) async fn handle_add_principle(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
) -> Result<String, String> {
    let value_name = args
        .get("value_name")
        .and_then(|v| v.as_str())
        .ok_or("missing 'value_name' argument")?;
    let principle = args
        .get("principle")
        .and_then(|v| v.as_str())
        .ok_or("missing 'principle' argument")?;
    let role = role_from_name(agent_name);
    let base = default_identity_dir().map_err(|e| format!("identity dir: {e}"))?;
    let mut index = load_identity(&base, role);
    if let Err(e) = add_principle(&mut index, value_name, principle) {
        return Err(identity_error(&e));
    }
    save_identity(&base, role, &index).map_err(|e| format!("save: {e}"))?;
    Ok(identity_success(&index))
}
