/// Read-side of the post-compact briefing pair (#287). Returns the
/// caller's own briefing by default; pass `role` to read another
/// role's (peer-read is allowed per the individuality-plus-emergent-
/// team discipline — transparency without merging).
///
/// Missing/empty file returns an empty string (not an error) so
/// callers can distinguish "no briefing yet" from "briefing is
/// unreadable" cleanly.
pub(in crate::bridge) async fn handle_get_briefing(
    args: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<String, String> {
    let default_role = name.split(':').next().unwrap_or(name);
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or(default_role);
    Ok(crate::session::briefing::load_briefing(role))
}
