// ── Story management tools (#140) ──────────────────────────────
// Server-side gh proxy with lane-aware permissions.

/// Agent-visible feature usage counts from the local session.
pub(in crate::bridge) async fn handle_get_usage(
    _args: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("cannot determine home directory")?;
    let path = home.join(".bpmux").join("usage.json");
    match std::fs::read_to_string(&path) {
        Ok(contents) => Ok(contents),
        Err(_) => Ok("{}".to_string()),
    }
}
