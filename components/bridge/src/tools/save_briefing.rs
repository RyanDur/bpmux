pub async fn handle_save_briefing(
    args: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<String, String> {
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or("missing 'content' argument")?;
    let role = name.split(':').next().unwrap_or(name);
    briefing::save_briefing(role, content).map_err(|e| format!("save_briefing({role}): {e}"))?;
    Ok(format!(
        "briefing saved for {role} ({} bytes)",
        content.len()
    ))
}
