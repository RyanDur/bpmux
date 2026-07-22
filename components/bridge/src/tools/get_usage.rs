pub async fn handle_get_usage(
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
