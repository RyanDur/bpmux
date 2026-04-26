// ── Story management tools (#140) ──────────────────────────────
// Server-side gh proxy with lane-aware permissions.

/// Agent-visible feature usage counts from the local session.
///
/// #486: read-side honors `BPMUX_USAGE_PATH` env-var override
/// (mirrors the write-side at `session::usage::default_usage_path`).
/// Production-default unchanged when env-var unset; tests can isolate
/// via `BPMUX_USAGE_PATH=<tmp>/usage.json`.
pub(in crate::bridge) async fn handle_get_usage(
    _args: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let path = match std::env::var_os("BPMUX_USAGE_PATH") {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let home = dirs::home_dir().ok_or("cannot determine home directory")?;
            home.join(".bpmux").join("usage.json")
        }
    };
    match std::fs::read_to_string(&path) {
        Ok(contents) => Ok(contents),
        Err(_) => Ok("{}".to_string()),
    }
}
