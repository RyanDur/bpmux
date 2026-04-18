/// Write-side of the post-compact briefing pair (#287). Agent curates
/// this mid-session; `session-start.sh` reads it post-compact so the
/// agent can re-enter context with their chosen continuity reminders.
pub(in crate::bridge) async fn handle_save_briefing(
    args: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<String, String> {
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or("missing 'content' argument")?;
    let role = name.split(':').next().unwrap_or(name);
    crate::session::briefing::save_briefing(role, content)
        .map_err(|e| format!("save_briefing({role}): {e}"))?;
    Ok(format!(
        "briefing saved for {role} ({} bytes)",
        content.len()
    ))
}
